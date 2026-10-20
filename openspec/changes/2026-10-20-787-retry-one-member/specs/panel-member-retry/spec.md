## ADDED Requirements

### Requirement: A retry names the members that run

`brokkr operator retry` SHALL accept `--member <name>`, repeatable. Each name SHALL resolve against the panel the carry applies to, spelled bare as the panel declares the member or as the tag the journal keys it with, `<step>:<member>` in a sequence, and SHALL be journaled resolved: the retry's `operator/commanded` SHALL carry `args.members`, the resolved tags in the order the names were given, spelled as `carried`, `settled_members` and the member markers spell them. Two names that resolve to one member SHALL be refused as that member given twice, naming the tag. A retry with members SHALL be disposed of as a retry is today: the fold's rule decides the acceptance, and the acceptance moves the run back to its phase's effect. A retry without `--member` SHALL journal `args: {}` and re-run every member, byte for byte as today. `--member` SHALL belong to `retry` alone and `--repo` to the member form alone, and the verb SHALL ask these two facts first, in this order, before every refusal it makes today: `--member` typed with any command word but `retry`, be it `stop`, `supersede`, `acknowledge` or a word that is no command, SHALL be refused naming `--member` as retry's alone; `--repo` typed without `--member` SHALL be refused naming `--repo` as the member form's alone; both before anything is written. The refusals the verb makes today, `supersede`'s own, the `supersede`-only flags on another command, `acknowledge` and the command word, SHALL follow in their existing order and keep their texts. `--repo` SHALL name the operated repository the head and the tree are read from, defaulting as `resume`'s `--repo` defaults: the root of the folder the command runs in, else the current directory, a relative path read against the current directory. The fold SHALL NOT read `args.members`; the engine SHALL read them for the effect the acceptance requested and for every bounded attempt of that effect, matching each tag exactly against the panel and resolving no name a second time. Naming every member of the panel SHALL be admitted exactly when the plain retry is admitted and the names resolve: nothing is carried, the parked attempt's shape, the head and the tree are not asked, and the attempt runs every member as a plain retry's does, its `effect/started` carrying no `carried`.

#### Scenario: One stuck reviewer

- **WHEN** a four-member review panel parked because `correctness` ran to no verdict, and the operator runs `brokkr operator retry --member correctness --reason "stuck reviewer"` on an unchanged head and a clean tree
- **THEN** the journal holds an `operator/commanded` with `command: "retry"` and `args: {"members": ["correctness"]}` followed by its `operator/accepted`, and the run folds to `running` at the review phase's effect

#### Scenario: The plain retry

- **WHEN** the operator runs `brokkr operator retry` with no `--member` on the same parked run
- **THEN** `operator/commanded` carries `args: {}`, the next attempt runs every member, and its `effect/started` carries no `carried`

#### Scenario: A member named on another command

- **WHEN** the operator runs `brokkr operator stop --member correctness`
- **THEN** the command is refused naming `--member` as retry's alone, and nothing is written

#### Scenario: A member named beside a supersede-only flag

- **WHEN** the operator runs `brokkr operator stop --member correctness --findings 1`
- **THEN** the command is refused naming `--member` as retry's alone, before the text that refuses `--findings` on a `stop` today, and nothing is written

#### Scenario: A member named on a word that is no command

- **WHEN** the operator runs `brokkr operator bogus --member correctness`
- **THEN** the command is refused naming `--member` as retry's alone, before the text that names the four commands, and nothing is written

#### Scenario: A supersede-only flag on a member retry

- **WHEN** the operator runs `brokkr operator retry --member correctness --findings 1`
- **THEN** the member facts pass, and the command is refused with the text that refuses `--findings` on a `retry` today

#### Scenario: A repository named on a plain retry

- **WHEN** the operator runs `brokkr operator retry --repo /work/tree` with no `--member`
- **THEN** the command is refused naming `--repo` as the member form's alone, and nothing is written

#### Scenario: The repository the member form reads

- **WHEN** the operator runs `brokkr operator retry --member correctness` without `--repo`, from a subdirectory of a repository that holds a `.brokkr` folder
- **THEN** the head and the tree are read at the folder's root, where `resume` operates without `--repo`

#### Scenario: A name given twice

- **WHEN** the operator runs `brokkr operator retry --member correctness --member correctness`
- **THEN** the command is refused naming `correctness` as given twice, and nothing is written

#### Scenario: A name spelled two ways

- **WHEN** the parked phase is the review sequence whose first step is the panel `positions`, and the operator runs `brokkr operator retry --member correctness --member positions:correctness`
- **THEN** the command is refused naming `positions:correctness` as given twice, and nothing is written

#### Scenario: The journal spells the resolved tag

- **WHEN** the parked phase is that sequence and the operator runs `brokkr operator retry --member correctness`
- **THEN** `operator/commanded` carries `args: {"members": ["positions:correctness"]}`, and the attempt's `carried` entries and its markers key every member `positions:<member>`

#### Scenario: Every member named

- **WHEN** the operator names every member of the panel on a parked attempt that records no verdict to carry, after the head moved and on a dirty tree
- **THEN** the command is admitted, `operator/commanded` carries `args.members` naming every member, every member runs, and the attempt's `effect/started` carries no `carried`

### Requirement: The carry applies to one panel

The panel SHALL be read from the manifest the journal pins at `run/started`: the body it declares for the parked phase, under the case the phase's latest `phase/entered` records when the seat selects. The carry SHALL apply to that body when it is a panel, or to a sequence's first step when that step is a panel and the parked attempt ended at it; the step an attempt ended at SHALL be read as the first step of the sequence with no `sequence-step-finished` marker in that attempt. A panel step after another step SHALL NOT carry: its members read `context.prior_results`, which the carrying attempt builds again and the requested digest does not cover; a sequence whose first step is no panel but a later step is SHALL be refused naming the panel step as following the first step, a fact of its own with its own text, asked after the body is found to run a panel and before any name is read. A sequence parked at a later step SHALL refuse `--member` naming the step the attempt ended at. Names SHALL resolve against that one panel: a name no member of it has, including a member of another panel step, SHALL be refused naming the panel's members. At drive time the engine SHALL refuse the same facts as a carry that no longer holds is refused. In a sequence the other steps SHALL run again, and the steps after the panel SHALL run on its aggregate.

#### Scenario: A sequence's first step is the panel

- **WHEN** the parked phase is a sequence of the panel step `positions` and the single step `chief`, parked with `positions:correctness` indeterminate and the three other positions settled, and the operator names `correctness`
- **THEN** `correctness` alone runs under `positions`, the three others are carried under the tags `positions:adversarial`, `positions:security` and `positions:spec-compliance`, `chief` runs again on the step's aggregate, and the attempt's markers name `positions:correctness` and `chief` alone

#### Scenario: A panel step after another step

- **WHEN** the parked phase is a sequence whose first step is a single seat and whose second step is a panel, parked with one member of that panel indeterminate, and the operator names that member
- **THEN** the command is refused naming the panel step as following the first step, whose result its members read, the name is not resolved, and nothing is written

#### Scenario: A sequence parked at the chief

- **WHEN** the review sequence parked because `chief` ran to no verdict after `positions` aggregated, its `sequence-step-finished` marker naming `positions`, and the operator names a member of `positions`
- **THEN** the command is refused naming `chief` as the step the attempt ended at, nothing is written, and the plain retry is admitted

#### Scenario: A name two panel steps share

- **WHEN** the parked phase is a sequence of two panel steps that both declare a member `security`, parked at the first, and the operator names `security`
- **THEN** the name resolves to the first step's `security` and nothing else, and the carry proceeds for the first step's panel

### Requirement: A verdict is carried only when nothing it judged has changed

A member's settled verdict SHALL be carried into the retry's attempt only when four facts hold: the reviewed head, the working tree, the pinned bundle and the member's inputs. The reviewed head SHALL be the `head` the parked phase's latest `phase/entered` recorded, checked against the commit-id shape before git reads it, compared with the head of the repository `--repo` names, else the repository `resume` would operate; a phase whose latest entry recorded no head, a recorded head that is not a commit id, or a repository whose head cannot be read SHALL refuse the carry. The working tree SHALL read `clean` at the two ends of the parked attempt and of every attempt a carried verdict is read from, as their terminal facts' `tree` record, and `clean` when the command is given and when the attempt is driven, every one of those readings taken through the one reader of `working-tree-reading`; any other reading at any of those observations, `dirty`, `refused` or `unrun`, SHALL refuse the carry naming the observation and the reading. The files the repository ignores are not read: an edit to one is the carry's accepted residual, as it is the ship gate's. The pinned bundle SHALL be held by the manifest gate `resume` applies before it builds any input. The member's inputs SHALL be held by rebuilding the seat input for the source attempt's effect and comparing its digest with the `input_digest` that effect's `effect/requested` recorded. The verb SHALL check the head, the tree and the journal's facts when the command is given and SHALL refuse with the reason, writing nothing. The engine SHALL check all four before anything of the attempt spawns, and a carry that no longer holds SHALL start the attempt on no driver and end it `effect/indeterminate` with a reason that opens with `PANEL-CARRY-REFUSED`, one space, and the refusal as compact JSON, so the run parks for the operator. The refusal SHALL be one typed vocabulary, spoken by the verb through a typed error whose text is pinned once and encoded by the engine as the marker's evidence.

#### Scenario: The head moved before the command

- **WHEN** a commit lands on the reviewed branch after the panel parked, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the recorded head and the head found, the journal holds no `operator/commanded` for it, and `brokkr operator retry` without members is admitted and re-runs the whole panel

#### Scenario: The head moved between the acceptance and the drive

- **WHEN** the member retry was accepted on an unchanged head, a commit lands, and `brokkr resume` drives the run
- **THEN** the attempt's `effect/started` names `driver: "none"` and no `carried`, its `effect/indeterminate` reason opens with `PANEL-CARRY-REFUSED` and its evidence names both heads, the run is `awaiting_operator`, and no member spawned

#### Scenario: The tree is dirty at the command

- **WHEN** the head is unchanged, a file under the operated repository is edited and not committed, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the working tree as `dirty` at the command, and nothing is written

#### Scenario: The tree was dirty while the panel judged

- **WHEN** the parked attempt's terminal fact records `tree.at_end` as `dirty`, the edits were reverted after the park, and the operator runs `brokkr operator retry --member correctness` on a clean tree
- **THEN** the command is refused naming the tree as `dirty` when the parked attempt ended, and nothing is written

#### Scenario: The tree at a carrying attempt's end

- **WHEN** the parked attempt is itself a carrying attempt whose terminal fact records `tree.at_end` as `dirty`, while the attempt its entries name records `tree` clean at both ends
- **THEN** the command is refused naming the tree as `dirty` when the parked attempt ended, and nothing is written

#### Scenario: Git answers the head and refuses status

- **WHEN** the head is unchanged and recorded, the operated repository's index file is made unreadable so that `git rev-parse HEAD` answers and `git status --porcelain` exits 128 printing nothing, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the working tree as `refused` at the command, not the head, and nothing is written

#### Scenario: Git refused status while the panel judged

- **WHEN** the parked attempt's terminal fact records `tree.at_start` as `refused`, and the operator runs `brokkr operator retry --member correctness` on a clean tree and an unchanged head
- **THEN** the command is refused naming the tree as `refused` when the parked attempt started, and nothing is written

#### Scenario: An ignored file edited after the park

- **WHEN** the head is unchanged, a file the repository's `.gitignore` names is edited after the park, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is admitted: the reader reads `clean`, and the edit is the accepted residual the proposal names

#### Scenario: A member's inputs changed

- **WHEN** a file the seat input embeds, such as the realm's house rules, is edited after the park, the head and the tree are as they were, and `brokkr resume` drives an accepted member retry
- **THEN** the attempt ends `effect/indeterminate` with `PANEL-CARRY-REFUSED` whose evidence names the source attempt's effect as the one whose input no longer rebuilds to its recorded digest, and the run parks

#### Scenario: Nothing changed

- **WHEN** the head, the tree, the bundle and every input are as the panel judged them, and `brokkr resume` drives the accepted member retry
- **THEN** the attempt's `effect/started` names every carried verdict, and only the named members spawn

#### Scenario: No head recorded

- **WHEN** the parked phase is a work panel in a phase that is not the protected one, whose `phase/entered` recorded no head
- **THEN** `--member` is refused naming the phase as having recorded no head, and nothing is written

#### Scenario: A recorded head that is not a commit id

- **WHEN** the parked phase's latest `phase/entered.head` reads as anything but forty lowercase hex digits
- **THEN** `--member` is refused naming the recorded head as not a commit id, git is not asked, and nothing is written

#### Scenario: A repository whose head cannot be read

- **WHEN** `--repo` names a directory that is not a git repository
- **THEN** `--member` is refused naming the repository as answering no head, the tree is not asked, and nothing is written

### Requirement: What is not carried refuses by name, in one order, before anything is written

With `--member`, the verb SHALL fold the run before it writes and SHALL ask its checks in this order, each fact with one text, the first fact that fails being the refusal reported, and a refused command writing nothing:

1. the arguments' shape, two facts in order: `--member` with a command word other than `retry`; `--repo` without `--member`; then the refusals the verb makes today, in their existing order;
2. the run's state by the fold's one rule (`acceptance_refusal`: not parked, no phase to retry), reported in the fold's words;
3. the panel, two facts in order: the parked phase's body, read as the previous requirement reads it, is a panel or a sequence with a panel step, else the command SHALL be refused naming the phase as running no panel; the panel is the body or the sequence's first step, else the command SHALL be refused naming the panel step as following the first step;
4. the names: each SHALL resolve to a member of that panel, else the command SHALL be refused naming the panel's members, and no member SHALL be given twice; when every member is named the command SHALL be admitted here and nothing below asked;
5. the parked attempt's shape, four facts in this order, each refused naming its own fact: the phase's current visit holds no attempt its panel ended, naming the phase and its visit; the attempt's terminal fact is `effect/succeeded`, naming the rule that parked; it ended at a step after the panel, naming that step; it records no verdict to carry, neither `settled_members` on its terminal fact nor `carried` on its `effect/started`, naming the attempt;
6. the unnamed members: every member not named SHALL have a verdict to carry, settled in the parked attempt or carried by it from an attempt whose `settled_members` holds it, else the command SHALL be refused naming every such member in the panel's declared order, in one refusal;
7. the head, as the previous requirement reads it;
8. the tree, as the previous requirement reads it.

The parked attempt SHALL be the latest attempt of the parked phase's current visit, before the retry's own effect, that its panel ended: its terminal fact is `effect/succeeded`; or its terminal fact is an `effect/indeterminate` or `effect/failed` that records `tree`, which only the panel writes; or, in a sequence, the attempt journals the panel step's `sequence-step-finished` marker. Every other attempt of the visit SHALL be passed over: one that started on no driver, one refused admission or provisioning before any member spawned, one the engine ended for a restart in flight, for journal contention or for a held outcome the fence refused, one a gate ended by moving the head, and one journaled before this change. At the command the parked attempt is read among every attempt of the visit; at drive time it is read among the attempts before the effect the acceptance requested, so every bounded attempt of that effect reads the one its first attempt read. Once the command is written only the fold's race refusal, a lost fence, SHALL follow, journaled as today. The engine SHALL refuse the same facts at drive time, in the same order with the inputs last, as the previous requirement refuses a carry that no longer holds.

#### Scenario: A member retry on a run that is not parked

- **WHEN** the operator runs `brokkr operator retry --member correctness` on a running run
- **THEN** the command is refused in the fold's words for a run not awaiting the operator, and the journal holds no `operator/commanded` for it, where the plain retry would journal its command and its `operator/rejected`

#### Scenario: The parked phase runs no panel

- **WHEN** the run parked at a single-driver seat, or at a sequence of single steps, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the phase as running no panel, nothing is written, and the plain retry is admitted

#### Scenario: A name the panel does not have

- **WHEN** the operator names `chief`, the review sequence's single step, or a name no member has
- **THEN** the command is refused naming the panel's members, and the parked attempt is not read

#### Scenario: No attempt its panel ended

- **WHEN** the parked phase's current visit holds no attempt its panel ended: its attempts started on no driver, were refused admission before any member spawned, or were journaled before this change
- **THEN** `--member` is refused naming the phase's visit as holding no attempt its panel ended, and nothing is written

#### Scenario: The shape facts in order

- **WHEN** the parked attempt's panel succeeded and a rule parked the run, and the operator names a member
- **THEN** the command is refused naming the attempt as succeeded and the rule that parked, the step and the record are not asked, nothing is written, and the plain retry is admitted

#### Scenario: A gate moved the head in the carrying attempt

- **WHEN** a member retry's attempt carried three verdicts and the retried member, a gate, committed, so the attempt ended with the `GATE-MOVED-HEAD` indeterminate, and the operator runs `brokkr operator retry --member correctness` on the moved head
- **THEN** the gate's attempt is passed over, the attempt before it that its panel ended is read, and the command is refused naming the recorded head and the head found

#### Scenario: The head is put back after a gate moved it

- **WHEN** the operator resets the branch to the recorded head after that park, the tree is clean, and runs `brokkr operator retry --member correctness`
- **THEN** the gate's attempt is passed over, the command is admitted, and the next attempt's `carried` names the attempt before the gate's

#### Scenario: A carrying attempt refused admission

- **WHEN** a member retry's attempt journaled `carried` on its `effect/started` and was then refused admission, its `effect/failed` written before any member spawned, the effect's bounded attempts were refused the same way and the run parked, and the operator runs `brokkr operator retry --member correctness` again once the refusal is put right
- **THEN** every refused attempt is passed over, the attempt before them that its panel ended is read, the command is admitted, and the next attempt's `carried` names that attempt, the one the refused attempts' entries also named

#### Scenario: The bounded attempt after a refused admission

- **WHEN** a carrying effect's first attempt is refused admission under a seat whose `max_attempts` is 2
- **THEN** the effect's second attempt reads the parked attempt among the attempts before the effect's request, carries the same entries from the same source attempts, and runs the named members again

#### Scenario: The engine restarted during the carrying attempt

- **WHEN** a member retry's attempt was in flight when the engine restarted, so its terminal fact is the restart's `effect/indeterminate` with no `tree`, and the operator runs `brokkr operator retry --member correctness` on an unchanged head and a clean tree
- **THEN** the restarted attempt is passed over, the command is admitted, and the next attempt's `carried` names the attempt the restarted one carried from

#### Scenario: A journal written before this change

- **WHEN** the parked phase's only attempt is a panel attempt whose terminal fact carries no `tree`, written before this change
- **THEN** `--member` is refused naming the phase's visit as holding no attempt its panel ended, and nothing is written

#### Scenario: An all-indeterminate panel

- **WHEN** every member of the parked attempt ran to no verdict, so its terminal fact carries `tree` and no `settled_members`, and the operator names one member
- **THEN** `--member` is refused naming the parked attempt as recording no verdict to carry, and nothing is written

#### Scenario: An unnamed member that never settled

- **WHEN** `correctness` and `security` both ran to no verdict and the operator names only `correctness`
- **THEN** the command is refused naming `security` as having no verdict to carry, and nothing is written

#### Scenario: Several unnamed members never settled

- **WHEN** `adversarial`, `correctness` and `security` all ran to no verdict, `spec-compliance` settled, and the operator names `correctness`
- **THEN** the command is refused once, naming `adversarial` and `security` in the panel's declared order as having no verdict to carry, and nothing is written

#### Scenario: The same member stuck twice

- **WHEN** the first attempt parked with `correctness` indeterminate, the member retry's attempt carried the three others from it and `correctness` ran to no verdict again, and the operator runs `brokkr operator retry --member correctness` on an unchanged head and a clean tree
- **THEN** the carrying attempt is read as the parked attempt, the three verdicts are found through its `carried` entries, the tree is checked at the carrying attempt's two ends and at the first attempt's, the command is admitted, and the third attempt's `carried` names the three by the first attempt

#### Scenario: A refused carry is not forfeited

- **WHEN** a member retry's drive ended `PANEL-CARRY-REFUSED` because the tree was dirty, the operator cleans the tree, and runs `brokkr operator retry --member correctness` again
- **THEN** the refusing attempt is passed over, the carry reads the attempt before it, the command is admitted, and the next attempt's `carried` names that earlier attempt

#### Scenario: Several refusals hold

- **WHEN** the operator names a member the panel does not have on a run whose head has also moved
- **THEN** the command is refused for the name alone, and the head is not read

### Requirement: The carrying attempt runs the named members and aggregates as a fresh panel

In the retry's attempt only the named members SHALL run; every other member's verdict SHALL be carried from the attempt it settled in, read from the parked attempt alone: settled there, its bytes in the parked attempt's `settled_members`, or named by that attempt's own `carried` entry, its bytes in the named attempt's `settled_members`. The declared aggregate SHALL join carried and fresh verdicts exactly as it joins a fresh panel's, over the members' bare names, so the attempt's result, its `inputs` and its `notes` SHALL equal the bytes a fresh panel aggregates over the same verdicts, and policy SHALL route them unchanged. Every per-site record and plan of the carrying attempt SHALL cover the members that run and no other: `effect/started.provenance`, `effect/started.boundary`, the `driver` label, the session offers, the admission claim and `panel-member-finished`. A fresh member that is indeterminate SHALL park the attempt and one that fails SHALL fail it; the bounded retry of that effect SHALL read the parked attempt among the attempts before the effect's request, SHALL run the named members again with the same carried entries from the same source attempts, and SHALL NOT carry a verdict that settled in an earlier bounded attempt of the same effect. The carry SHALL be the same inside one effect as a fresh panel's (decision 0002).

#### Scenario: The same aggregate as a full re-run

- **WHEN** a review panel of `adversarial`, `correctness`, `security` and `spec-compliance` parked with `correctness` indeterminate and the three others settled, and the member retry runs `correctness` to a verdict
- **THEN** the attempt's `effect/succeeded.result` equals, byte for byte, the result a fresh panel aggregates over the same four verdicts: the worst verdict, the joined `inputs`, and `notes` holding every member's notes and verdict

#### Scenario: A residual from the retried member routes as today

- **WHEN** the retried member reports `residual` at severity `low` and the carried verdicts are `clean`
- **THEN** the panel's result is `residual` with `max_residual_severity` `low`, the table's existing rule routes it, and the review it leads to runs every member fresh

#### Scenario: The fresh member fails and the effect retries

- **WHEN** the named member's driver fails to start in the retry's first attempt, under a seat whose `max_attempts` is 2
- **THEN** that attempt's `effect/failed` records no settled member, its `effect/started` having named the carried verdicts, and the effect's second attempt runs the named member again with `carried` entries naming the same source attempts

#### Scenario: A member that settled in the first bounded attempt is not carried into the second

- **WHEN** the member retry names `correctness` and `security`, `security` settles in the effect's first attempt and `correctness`'s driver fails, under a seat whose `max_attempts` is 2
- **THEN** the effect's second attempt runs `correctness` and `security` again, its `carried` naming the two others alone, by the attempts the first attempt named

#### Scenario: Only the members that run are recorded

- **WHEN** the member retry's attempt carries three verdicts and runs `correctness`
- **THEN** its `effect/started.provenance` and `boundary` hold `correctness` alone, its `driver` label counts one member, one session is offered, admission claims one site, and one `panel-member-finished` is journaled
