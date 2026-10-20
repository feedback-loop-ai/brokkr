## ADDED Requirements

### Requirement: A retry names the members that run

`brokkr operator retry` SHALL accept `--member <name>`, repeatable, each naming a member of the panel the parked phase's latest attempt ran, as the panel declares the name; in a sequence the journal's tag `<step>:<member>` SHALL be accepted too, and a bare name two panel steps of that attempt share SHALL be refused naming both tags. A retry with members SHALL journal them on its `operator/commanded` as `args.members`, in the order named, and SHALL be disposed of as a retry is today: the fold's rule decides the acceptance, and the acceptance moves the run back to its phase's effect. A retry without `--member` SHALL journal `args: {}` and re-run every member, byte for byte as today. `--member` SHALL belong to `retry` alone: typed with `stop`, `supersede` or `acknowledge` it SHALL be refused before anything is written. The fold SHALL NOT read `args.members`; the engine SHALL read them for the effect the acceptance requested and for every bounded attempt of that effect.

#### Scenario: One stuck reviewer

- **WHEN** a four-member review panel parked because `correctness` ran to no verdict, and the operator runs `brokkr operator retry --member correctness --reason "stuck reviewer"` on an unchanged head
- **THEN** the journal holds an `operator/commanded` with `command: "retry"` and `args: {"members": ["correctness"]}` followed by its `operator/accepted`, and the run folds to `running` at the review phase's effect

#### Scenario: The plain retry

- **WHEN** the operator runs `brokkr operator retry` with no `--member` on the same parked run
- **THEN** `operator/commanded` carries `args: {}`, the next attempt runs every member, and its `effect/started` carries no `carried`

#### Scenario: A member named on another command

- **WHEN** the operator runs `brokkr operator stop --member correctness`
- **THEN** the command is refused naming `--member` as retry's alone, and nothing is written

### Requirement: A verdict is carried only when nothing it judged has changed

A member's settled verdict SHALL be carried into the retry's attempt only when three facts are byte-identical to what it judged: the reviewed head, the pinned bundle and the member's inputs. The reviewed head SHALL be the `head` the parked phase's latest `phase/entered` recorded, checked against the commit-id shape before git reads it, compared with the head of the repository (`--repo`, else the current directory, as `resume` reads it); a phase whose latest entry recorded no head, or a repository whose head cannot be read, SHALL refuse the carry. The pinned bundle SHALL be held by the manifest gate `resume` applies before it builds any input. The member's inputs SHALL be held by rebuilding the seat input for the source attempt's effect and comparing its digest with the `input_digest` that effect's `effect/requested` recorded. The verb SHALL check the head and the journal's facts when the command is given and SHALL refuse with the reason, writing nothing. The engine SHALL check all three before anything of the attempt spawns, and a carry that no longer holds SHALL start the attempt on no driver and end it `effect/indeterminate` with a reason that opens with `PANEL-CARRY-REFUSED` and carries the refusal's evidence as JSON, so the run parks for the operator. The refusal SHALL be a typed error whose text is pinned once.

#### Scenario: The head moved before the command

- **WHEN** a commit lands on the reviewed branch after the panel parked, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the recorded head and the head found, the journal holds no `operator/commanded` for it, and `brokkr operator retry` without members is admitted and re-runs the whole panel

#### Scenario: The head moved between the acceptance and the drive

- **WHEN** the member retry was accepted on an unchanged head, a commit lands, and `brokkr resume` drives the run
- **THEN** the attempt's `effect/started` names `driver: "none"` and no `carried`, its `effect/indeterminate` reason opens with `PANEL-CARRY-REFUSED` and names both heads, the run is `awaiting_operator`, and no member spawned

#### Scenario: A member's inputs changed

- **WHEN** a file the seat input embeds, such as the realm's house rules, is edited after the park, the head is unchanged, and `brokkr resume` drives an accepted member retry
- **THEN** the attempt ends `effect/indeterminate` with `PANEL-CARRY-REFUSED` naming the source attempt's effect whose input no longer rebuilds to its recorded digest, and the run parks

#### Scenario: Nothing changed

- **WHEN** the head, the bundle and every input are as the panel judged them, and `brokkr resume` drives the accepted member retry
- **THEN** the attempt's `effect/started` names every carried verdict, and only the named members spawn

### Requirement: What is not carried refuses by name

A member that never settled, whose outcome in the parked attempt is indeterminate or failed, SHALL NOT be carried: left unnamed, the command SHALL refuse naming that member. A name the parked attempt's panel does not have SHALL be refused naming the panel's members. A parked phase whose latest attempt ran no panel, whose latest attempt succeeded, or whose latest attempt's terminal fact records no settled verdict SHALL refuse `--member` by that fact, and the plain retry SHALL stand. The engine SHALL refuse the same facts at drive time, as the previous requirement refuses a moved head.

#### Scenario: An unnamed member that never settled

- **WHEN** `correctness` and `security` both ran to no verdict and the operator names only `correctness`
- **THEN** the command is refused naming `security` as having no settled verdict to carry, and nothing is written

#### Scenario: A name the panel does not have

- **WHEN** the operator names `chief`, the review sequence's single step, or a name no member has
- **THEN** the command is refused naming the panel's members

#### Scenario: A parked phase that is not a panel

- **WHEN** the run parked at a single-driver seat, or at a panel whose latest attempt succeeded and parked by rule
- **THEN** `--member` is refused by that fact, nothing is written, and the plain retry is admitted

#### Scenario: A journal written before this change

- **WHEN** the parked attempt's terminal fact records no `settled_members`
- **THEN** every unnamed member is refused as having no settled verdict recorded

### Requirement: The carrying attempt runs the named members and aggregates as a fresh panel

In the retry's attempt only the named members SHALL run; every other member's verdict SHALL be carried from the attempt it settled in, read from the parked attempt alone: settled there, or named by that attempt's own `carried` entry. The declared aggregate SHALL join carried and fresh verdicts exactly as it joins a fresh panel's, so the attempt's result, its `inputs` and its `notes` SHALL equal the bytes a fresh panel aggregates over the same verdicts, and policy SHALL route them unchanged. A fresh member that is indeterminate SHALL park the attempt and one that fails SHALL fail it; the bounded retry of that effect SHALL run the named members again with the same carried entries from the same source attempts, and an attempt of the carrying effect SHALL never be a source. `panel-member-finished` SHALL be journaled only for the members that ran. In a sequence the carry SHALL apply to the panel step whose members are named, the other steps SHALL run again, and the steps after it SHALL run on that step's aggregate. The carry SHALL be the same inside one effect as a fresh panel's (decision 0002).

#### Scenario: The same aggregate as a full re-run

- **WHEN** a review panel of `adversarial`, `correctness`, `security` and `spec-compliance` parked with `correctness` indeterminate and the three others settled, and the member retry runs `correctness` to a verdict
- **THEN** the attempt's `effect/succeeded.result` equals, byte for byte, the result a fresh panel aggregates over the same four verdicts: the worst verdict, the joined `inputs`, and `notes` holding every member's notes and verdict

#### Scenario: A residual from the retried member routes as today

- **WHEN** the retried member reports `residual` at severity `low` and the carried verdicts are `clean`
- **THEN** the panel's result is `residual` with `max_residual_severity` `low`, the table's existing rule routes it, and the review it leads to runs every member fresh

#### Scenario: The fresh member fails and the effect retries

- **WHEN** the named member's driver fails to start in the retry's first attempt, under a seat whose `max_attempts` is 2
- **THEN** that attempt's `effect/failed` records no settled member, its `effect/started` having named the carried verdicts, and the effect's second attempt runs the named member again with `carried` entries naming the same source attempts

#### Scenario: A sequence's panel step

- **WHEN** the parked phase is a sequence of the panel step `positions` and the single step `chief`, parked with one member of `positions` indeterminate, and the operator names that member
- **THEN** the named member runs under `positions`, the others are carried as `positions:<member>` from the parked attempt, `chief` runs again on the step's aggregate, and the attempt's markers name only the member that ran

#### Scenario: Every member named

- **WHEN** the operator names every member of the panel
- **THEN** every member runs and `effect/started` carries no `carried`
