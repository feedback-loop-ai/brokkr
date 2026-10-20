## ADDED Requirements

### Requirement: A retry names the members that run

`brokkr operator retry` SHALL accept `--member <name>`, repeatable, each naming a member of the panel the carry applies to, as the panel declares the name; in a sequence the journal's tag `<step>:<member>` SHALL be accepted too. A name given twice SHALL be refused. A retry with members SHALL journal them on its `operator/commanded` as `args.members`, in the order named, and SHALL be disposed of as a retry is today: the fold's rule decides the acceptance, and the acceptance moves the run back to its phase's effect. A retry without `--member` SHALL journal `args: {}` and re-run every member, byte for byte as today. `--member` SHALL belong to `retry` alone: typed with `stop`, `supersede` or `acknowledge` it SHALL be refused before anything is written. `--repo` SHALL name the operated repository the head and the tree are read from, defaulting to the current directory as `resume` reads it, and SHALL belong to the member form alone: typed without `--member` it SHALL be refused before anything is written. The fold SHALL NOT read `args.members`; the engine SHALL read them for the effect the acceptance requested and for every bounded attempt of that effect. Naming every member of the panel SHALL carry nothing and SHALL check no identity fact: the attempt runs every member as a plain retry's does.

#### Scenario: One stuck reviewer

- **WHEN** a four-member review panel parked because `correctness` ran to no verdict, and the operator runs `brokkr operator retry --member correctness --reason "stuck reviewer"` on an unchanged head and a clean tree
- **THEN** the journal holds an `operator/commanded` with `command: "retry"` and `args: {"members": ["correctness"]}` followed by its `operator/accepted`, and the run folds to `running` at the review phase's effect

#### Scenario: The plain retry

- **WHEN** the operator runs `brokkr operator retry` with no `--member` on the same parked run
- **THEN** `operator/commanded` carries `args: {}`, the next attempt runs every member, and its `effect/started` carries no `carried`

#### Scenario: A member named on another command

- **WHEN** the operator runs `brokkr operator stop --member correctness`
- **THEN** the command is refused naming `--member` as retry's alone, and nothing is written

#### Scenario: A repository named on a plain retry

- **WHEN** the operator runs `brokkr operator retry --repo /work/tree` with no `--member`
- **THEN** the command is refused naming `--repo` as the member form's alone, and nothing is written

#### Scenario: A name given twice

- **WHEN** the operator runs `brokkr operator retry --member correctness --member correctness`
- **THEN** the command is refused naming `correctness` as given twice, and nothing is written

#### Scenario: Every member named

- **WHEN** the operator names every member of the panel
- **THEN** every member runs, `effect/started` carries no `carried`, and the command is admitted whether or not the head or the tree moved

### Requirement: The carry applies to one panel

The carry SHALL apply to the seat's body when it is a panel, or to a sequence's first step when that step is a panel and the parked attempt ended at it. A panel step after another step SHALL NOT carry: its members read `context.prior_results`, which the carrying attempt builds again and the requested digest does not cover. A sequence parked at a later step SHALL refuse `--member` naming the step the attempt ended at. Names SHALL resolve against that one panel: a name no member of it has, including a member of another panel step, SHALL be refused naming the panel's members. At drive time the engine SHALL refuse the same facts as a carry that no longer holds is refused. In a sequence the other steps SHALL run again, and the steps after the panel SHALL run on its aggregate.

#### Scenario: A sequence's first step is the panel

- **WHEN** the parked phase is a sequence of the panel step `positions` and the single step `chief`, parked with `positions:correctness` indeterminate and the three other positions settled, and the operator names `correctness`
- **THEN** `correctness` alone runs under `positions`, the three others are carried under the tags `positions:adversarial`, `positions:security` and `positions:spec-compliance`, `chief` runs again on the step's aggregate, and the attempt's markers name `positions:correctness` and `chief` alone

#### Scenario: A panel step after another step

- **WHEN** the parked phase is a sequence whose first step is a single seat and whose second step is a panel, parked with one member of that panel indeterminate, and the operator names that member
- **THEN** the command is refused naming the panel step as following the first step, whose result its members read, and nothing is written

#### Scenario: A sequence parked at the chief

- **WHEN** the review sequence parked because `chief` ran to no verdict after `positions` aggregated, and the operator names a member of `positions`
- **THEN** the command is refused naming `chief` as the step the attempt ended at, nothing is written, and the plain retry is admitted

#### Scenario: A name two panel steps share

- **WHEN** the parked phase is a sequence of two panel steps that both declare a member `security`, parked at the first, and the operator names `security`
- **THEN** the name resolves to the first step's `security` and nothing else, and the carry proceeds for the first step's panel

### Requirement: A verdict is carried only when nothing it judged has changed

A member's settled verdict SHALL be carried into the retry's attempt only when four facts hold: the reviewed head, the working tree, the pinned bundle and the member's inputs. The reviewed head SHALL be the `head` the parked phase's latest `phase/entered` recorded, checked against the commit-id shape before git reads it, compared with the head of the repository `--repo` names, else the current directory; a phase whose latest entry recorded no head, a recorded head that is not a commit id, or a repository whose head cannot be read SHALL refuse the carry. The working tree SHALL be clean, read as `git status --porcelain` reporting nothing and an unreadable repository counting as dirty, when the parked attempt started and ended, as its terminal fact's `tree` records, and when the command is given and when the attempt is driven. The pinned bundle SHALL be held by the manifest gate `resume` applies before it builds any input. The member's inputs SHALL be held by rebuilding the seat input for the source attempt's effect and comparing its digest with the `input_digest` that effect's `effect/requested` recorded. The verb SHALL check the head, the tree and the journal's facts when the command is given and SHALL refuse with the reason, writing nothing. The engine SHALL check all four before anything of the attempt spawns, and a carry that no longer holds SHALL start the attempt on no driver and end it `effect/indeterminate` with a reason that opens with `PANEL-CARRY-REFUSED`, one space, and the refusal as compact JSON, so the run parks for the operator. The refusal SHALL be one typed vocabulary, spoken by the verb through a typed error whose text is pinned once and encoded by the engine as the marker's evidence.

#### Scenario: The head moved before the command

- **WHEN** a commit lands on the reviewed branch after the panel parked, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the recorded head and the head found, the journal holds no `operator/commanded` for it, and `brokkr operator retry` without members is admitted and re-runs the whole panel

#### Scenario: The head moved between the acceptance and the drive

- **WHEN** the member retry was accepted on an unchanged head, a commit lands, and `brokkr resume` drives the run
- **THEN** the attempt's `effect/started` names `driver: "none"` and no `carried`, its `effect/indeterminate` reason opens with `PANEL-CARRY-REFUSED` and its evidence names both heads, the run is `awaiting_operator`, and no member spawned

#### Scenario: The tree is dirty at the command

- **WHEN** the head is unchanged, a file under the operated repository is edited and not committed, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the working tree as dirty, and nothing is written

#### Scenario: The tree was dirty while the panel judged

- **WHEN** the parked attempt's terminal fact records `tree.at_end` as `dirty`, the edits were reverted after the park, and the operator runs `brokkr operator retry --member correctness` on a clean tree
- **THEN** the command is refused naming the tree as dirty when the parked attempt ended, and nothing is written

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
- **THEN** `--member` is refused naming the repository as answering no head, and nothing is written

### Requirement: What is not carried refuses by name, in one order, before anything is written

With `--member`, the verb SHALL fold the run before it writes and SHALL ask its checks in this order: the arguments' shape; the run's state by the fold's one rule (`acceptance_refusal`: not parked, no phase to retry), reported in the fold's words; the parked attempt's shape; the names; the head; the tree. The first check that fails SHALL be the refusal reported, each check SHALL have one text, and a refused command SHALL write nothing. The parked attempt SHALL be the latest attempt of the parked phase's current visit that started on a driver: an attempt that started on none SHALL be passed over. A member that never settled, whose outcome in the parked attempt is indeterminate or failed, SHALL NOT be carried: left unnamed, the command SHALL refuse naming that member. A parked attempt that ran no panel, whose panel succeeded, that a gate ended by moving the head (`GATE-MOVED-HEAD`), or whose terminal fact records no `settled_members` SHALL refuse `--member` by that fact, and the plain retry SHALL stand. Once the command is written only the fold's race refusal, a lost fence, SHALL follow, journaled as today. The engine SHALL refuse the same facts at drive time, in the same order with the inputs last, as the previous requirement refuses a carry that no longer holds.

#### Scenario: A member retry on a run that is not parked

- **WHEN** the operator runs `brokkr operator retry --member correctness` on a running run
- **THEN** the command is refused in the fold's words for a run not awaiting the operator, and the journal holds no `operator/commanded` for it, where the plain retry would journal its command and its `operator/rejected`

#### Scenario: An unnamed member that never settled

- **WHEN** `correctness` and `security` both ran to no verdict and the operator names only `correctness`
- **THEN** the command is refused naming `security` as having no settled verdict to carry, and nothing is written

#### Scenario: A name the panel does not have

- **WHEN** the operator names `chief`, the review sequence's single step, or a name no member has
- **THEN** the command is refused naming the panel's members

#### Scenario: A parked phase that is not a panel

- **WHEN** the run parked at a single-driver seat, or at a panel whose latest attempt succeeded and parked by rule
- **THEN** `--member` is refused by that fact, nothing is written, and the plain retry is admitted

#### Scenario: A gate moved the head in the parked attempt

- **WHEN** the parked attempt's terminal fact is the `GATE-MOVED-HEAD` indeterminate a gate wrote
- **THEN** `--member` is refused naming the gate that moved the head, nothing is written, and the plain retry is admitted

#### Scenario: A journal written before this change

- **WHEN** the parked attempt's terminal fact records no `settled_members`
- **THEN** every unnamed member is refused as having no settled verdict recorded

#### Scenario: A refused carry is not forfeited

- **WHEN** a member retry's drive ended `PANEL-CARRY-REFUSED` because the tree was dirty, the operator cleans the tree, and runs `brokkr operator retry --member correctness` again
- **THEN** the refusing attempt is passed over, the carry reads the attempt before it, the command is admitted, and the next attempt's `carried` names that earlier attempt

#### Scenario: Several refusals hold

- **WHEN** the operator names a member the panel does not have on a run whose head has also moved
- **THEN** the command is refused for the name alone, and the head is not read

### Requirement: The carrying attempt runs the named members and aggregates as a fresh panel

In the retry's attempt only the named members SHALL run; every other member's verdict SHALL be carried from the attempt it settled in, read from the parked attempt alone: settled there, or named by that attempt's own `carried` entry, its bytes read from the named attempt's `settled_members`. The declared aggregate SHALL join carried and fresh verdicts exactly as it joins a fresh panel's, over the members' bare names, so the attempt's result, its `inputs` and its `notes` SHALL equal the bytes a fresh panel aggregates over the same verdicts, and policy SHALL route them unchanged. Every per-site record and plan of the carrying attempt SHALL cover the members that run and no other: `effect/started.provenance`, `effect/started.boundary`, the `driver` label, the session offers, the admission claim and `panel-member-finished`. A fresh member that is indeterminate SHALL park the attempt and one that fails SHALL fail it; the bounded retry of that effect SHALL run the named members again with the same carried entries from the same source attempts, and an attempt of the carrying effect SHALL never be a source. The carry SHALL be the same inside one effect as a fresh panel's (decision 0002).

#### Scenario: The same aggregate as a full re-run

- **WHEN** a review panel of `adversarial`, `correctness`, `security` and `spec-compliance` parked with `correctness` indeterminate and the three others settled, and the member retry runs `correctness` to a verdict
- **THEN** the attempt's `effect/succeeded.result` equals, byte for byte, the result a fresh panel aggregates over the same four verdicts: the worst verdict, the joined `inputs`, and `notes` holding every member's notes and verdict

#### Scenario: A residual from the retried member routes as today

- **WHEN** the retried member reports `residual` at severity `low` and the carried verdicts are `clean`
- **THEN** the panel's result is `residual` with `max_residual_severity` `low`, the table's existing rule routes it, and the review it leads to runs every member fresh

#### Scenario: The fresh member fails and the effect retries

- **WHEN** the named member's driver fails to start in the retry's first attempt, under a seat whose `max_attempts` is 2
- **THEN** that attempt's `effect/failed` records no settled member, its `effect/started` having named the carried verdicts, and the effect's second attempt runs the named member again with `carried` entries naming the same source attempts

#### Scenario: Only the members that run are recorded

- **WHEN** the member retry's attempt carries three verdicts and runs `correctness`
- **THEN** its `effect/started.provenance` and `boundary` hold `correctness` alone, its `driver` label counts one member, one session is offered, admission claims one site, and one `panel-member-finished` is journaled
