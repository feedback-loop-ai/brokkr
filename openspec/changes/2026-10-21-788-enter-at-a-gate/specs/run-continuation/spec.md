## ADDED Requirements

### Requirement: An entered run names the runs it continues

`brokkr run --enter` SHALL accept `--continues <run>`, repeatable, naming the earlier runs whose work on the branch the entered run continues; without `--enter` the flag SHALL be refused. Each value SHALL resolve as `--run` does, a full id, a unique prefix or `latest`, in the journal the new run is written to. Each named run MUST exist and be concluded, `completed` or `stopped`; a `running` run and an `awaiting_operator` run SHALL be refused naming the run and its status, since the one is still at work and the other is resumed or concluded first (decision 0092 ruling 4). A run named twice SHALL be refused. Every refusal SHALL come before an event is written. An entry that names no run SHALL be admitted: a branch with no run to continue, entered at a gate, is a landing.

#### Scenario: Two stopped runs are continued

- **WHEN** runs `a` and `b` stopped after committing their work, and a run is started with `--enter verify --continues a --continues b`
- **THEN** the run starts, and its `run/started.entry.continues` lists `a` then `b`, each with the feature its own start recorded

#### Scenario: A parked or running run is not continued

- **WHEN** `--continues` names a run that is `awaiting_operator`, or one that is `running`
- **THEN** no run is created, and the refusal names the run, its status, and that it is resumed or concluded first

#### Scenario: A run named twice, or not at all

- **WHEN** `--continues` names the same run twice, or names a selector no run in the journal matches
- **THEN** no run is created, and the refusal names the selector and why

### Requirement: The journal records the entry and the chain

`run/started` SHALL carry one optional object, `entry`, holding the phase entered and `continues`, the continued runs in the order named, each with its `run_id` and `feature`. It SHALL be absent on a run started without an entry, so every journal written before and every ordinary run after is byte for byte what it was. It SHALL be typed once, at the edge, and the fold SHALL read nothing of it: `RunState` gains nothing, and it is an additive, optional, absent-by-default field under the vocabulary's amended rule. The engine SHALL read it at start and at resume to enter the entry instead of `initial`, so a run journaled but not yet entered resumes into its entry. Its numbered extension schema SHALL be minted when decision 0094 is accepted (decision 0082 ruling 1); until then the decision and the type are its shape.

#### Scenario: An ordinary run's start is unchanged

- **WHEN** a run is started without `--enter`
- **THEN** its `run/started` payload is byte for byte the payload the same recipe and feature produced before this change, and every witness digest stands

#### Scenario: A run journaled but not entered resumes into its entry

- **WHEN** an entered run's journal ends at `run/started`, the process having ended before `phase/entered`, and the run is resumed under its pinned bundle
- **THEN** the resumed run enters the entry phase, not `initial`

#### Scenario: An old journal folds as it did

- **WHEN** a journal written before `entry` existed is folded, verified and viewed
- **THEN** it folds, verifies and views exactly as before, with no entry shown

### Requirement: The run view names the gate entered and the runs continued

`RunView` SHALL derive once, from `run/started`, the entry: the gate and the continued runs, each led by its feature with its id after. The console's run header, `brokkr ui`, the TUI and `brokkr run`'s plain start line SHALL paint that derivation and derive nothing of their own; for a run without an entry nothing SHALL be painted. The fleet list is unchanged.

#### Scenario: The header names the chain

- **WHEN** the run view of a run entered at `verify` continuing runs `a` and `b` is read on any surface
- **THEN** it names `verify` as the gate entered and the two runs, each by its feature and then its id, in the order named

#### Scenario: A run entered at a gate over a branch with no run

- **WHEN** the run view of a run entered at `review` with no `--continues` is read
- **THEN** it names `review` as the gate entered and says the run continues no earlier run

### Requirement: The contribution gate reads the chain and still vouches

`scripts/delivered-by-brokkr.sh` SHALL vouch an entered run exactly as it vouches any run: the ship anchor's per-file patch map is cut from the merge-base of the default branch and the vouched head, so it spans every commit on the branch whichever run or hand made it, and the tier is cut by the delta since that judgment (decision 0038). The gate SHALL read `entry` from the vouching run's verified journal and name the gate entered and each continued run's id on its check line. An `entry` whose shape it cannot read, not an object, a phase that is not a word, or `continues` not an array of objects whose `run_id` is a native run id, SHALL be a refusal, never a vouch; an absent `entry` SHALL read as today. The gate SHALL fetch no continued run's evidence.

#### Scenario: An entered run vouches for a branch two runs built

- **WHEN** run `a` committed an implementation and stopped, run `b` entered at `verify` continuing `a` remediated once and shipped, and the pull request head is the head `b` anchored
- **THEN** the gate passes at tier `vouched` and its line names run `b`, the gate `verify`, and run `a`

#### Scenario: A head edited after the entered run's judgment

- **WHEN** a code path is committed after run `b` shipped
- **THEN** the gate refuses with the code delta, as it would for any run

#### Scenario: A chain the gate cannot read

- **WHEN** the vouching run's `run/started.entry` is a string, or lists a continued run without a native run id
- **THEN** the gate refuses, naming the run and the unreadable entry, and the operator's by-hand label is the only way past, logged as every skip is
