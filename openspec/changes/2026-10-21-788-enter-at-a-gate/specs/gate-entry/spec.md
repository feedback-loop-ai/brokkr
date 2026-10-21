## ADDED Requirements

### Requirement: A run enters at a gate of its table

`brokkr run` SHALL accept `--enter <phase>`, and a run started with it SHALL enter the named phase first, instead of the table's `initial`. The bundle it runs under SHALL be the one the same recipe compiles to without an entry: the table, its `initial`, the manifest, its digest and the witnesses do not move, because an entry is a fact of the run, not of the recipe. A recipe whose own `initial` is a gate SHALL start as it always did when no entry is named, and its `initial` SHALL be an entry like any other phase when it passes admission. A Looper-bound run (`--dispatch`) SHALL refuse an entry by name, as it refuses a named map, until the v2 manifest lineage carries one.

#### Scenario: A finished branch is verified, judged and shipped without a work seat

- **WHEN** a recipe whose table lists `intake, design, implement, verify, review, ship, done, stop`, intake and design being work phases, is run with `--enter verify` over a repository whose branch holds the work a stopped run of the same recipe committed after entering intake, design and implement, and its verify, review and ship seats report `pass`, `clean`, `ready` and `shipped`
- **THEN** the run completes by `SHIP-COMPLETE`; its `phase/entered` events name `verify`, `review` and `ship` and never `intake`, `design` or `implement`; no effect is requested of a work seat; and the run manifest equals the manifest a run of the same recipe without an entry pins

#### Scenario: A landing starts where it always did, and its first phase is an entry

- **WHEN** `brokkr run --recipe landing` is started without `--enter`
- **THEN** the run enters `classify` first and its `run/started` carries no `entry`
- **AND WHEN** it is started with `--enter classify --continues <a stopped run>`
- **THEN** the run enters `classify` first and its `run/started.entry` records `classify` and the run named

#### Scenario: A Looper-bound run takes no entry

- **WHEN** `brokkr run` is given both `--dispatch` and `--enter`
- **THEN** the launch is refused naming both flags before anything is compiled, opened or written

### Requirement: Entry is admitted by six facts of the table and its seats

An entry SHALL be admitted when the compiled table and the entry satisfy all six, judged in this order:

1. the phase is one of the table's `phases` and is not a terminal, so it is seated;
2. its name matches `^[a-z0-9][a-z0-9._-]{0,63}$`, the grammar the contribution gate prints (`run-continuation`), so the compiler never admits an entry the gate refuses;
3. every driver-bearing site of that seat is a gate (its class under decision 0021 ruling 1 is `gate`; a sequence counts by its steps, each written `gate`, a dialect step included, since an undeclared site is work; a panel by its members; and a selector by every case and its default);
4. the protected phase is the entry itself or is unavoidable from it, by the walk the compiler already makes from `initial`, started at the entry;
5. no work phase (a seated phase fact 3 would refuse) is reached from the entry over forward rulings alone: a ruling is forward when its `next` stands after its `from` in the `phases` list, a return when before, a self-loop neither, and a park draws no edge;
6. no seat reached from the entry needs a journal fact only a skipped phase can set: a seat whose body selects on `strategy` with no `default`, reached without crossing the strategy phase (`triage`, decision 0041 ruling 6); or a dialect step that expands `{change}`, reached without crossing a seat that declares the `change` input, the steps of its own sequence counted in order.

Any other entry SHALL be refused when the recipe and the entry are compiled, before a run id is minted or an event is written, by a typed error that names the phase, the first fact it failed with its particulars (the work phase and the ruling for fact 5; the seat or the step and the fact for fact 6), and the entries the table admits, derived by the same six facts and listed in the order of the `phases` list, or that it admits none. The engine SHALL hold no roster of which phases may be entered: the facts are the table's and its seats'.

#### Scenario: A work phase is refused

- **WHEN** a `fast`-shaped recipe (`implement, verify, review, ship, done, stop`) is run with `--enter implement`
- **THEN** no run is created, and the refusal names `implement` as a work phase (fact 3) and lists `verify, review` as the entries the table admits, in that order

#### Scenario: A gate that reaches done without the protected phase is refused

- **WHEN** a `fast`-shaped recipe is run with `--enter ship`
- **THEN** no run is created, and the refusal names `ship` as a gate from which `done` is reached without `review` (fact 4), and lists `verify, review`

#### Scenario: The protected phase itself is an entry

- **WHEN** a `fast`-shaped recipe is run with `--enter review` and review reports `clean`
- **THEN** the run enters `review` first, and ship's drift check reads the head review was entered at

#### Scenario: A phase the table lacks, a terminal, or a name the gate cannot print is refused

- **WHEN** a `fast`-shaped recipe is run with `--enter intake`, or with `--enter done`, or a fixture table whose gate is named `verify gate` is run with `--enter "verify gate"`
- **THEN** no run is created, and the refusal names the phase as absent from the table (fact 1), as a terminal with no seat (fact 1), or as a name outside `^[a-z0-9][a-z0-9._-]{0,63}$` (fact 2), and lists the entries the table admits

#### Scenario: A dialect phase whose check step declares no class is a work phase

- **WHEN** a `triage`-shaped recipe is run with `--enter clarify`, or with `--enter analyze`
- **THEN** no run is created, and the refusal names the phase as a work phase, its `check` step declaring no class (fact 3), and says the table admits no entry

#### Scenario: A gate from which a work phase is reached forward is refused

- **WHEN** a `triage`-shaped recipe is run with `--enter triage`, or a fixture table shaped like it whose dialect steps are written `gate` is run with `--enter clarify` or `--enter analyze`
- **THEN** no run is created, and the refusal names `implement` reached forward by `TRIAGE-CHORE`, `design` reached forward by `CLARIFY-CLEAR`, or `implement` reached forward by `ANALYZE-CONSISTENT` (fact 5), and says the table admits no entry

#### Scenario: A road that needs a skipped phase's strategy is refused

- **WHEN** a `triage`-shaped recipe is run with `--enter verify` or `--enter review`
- **THEN** no run is created, and the refusal names `review` as a seat that selects on `strategy` with no `default` while `triage`, which rules it, is skipped (fact 6), and says the table admits no entry

#### Scenario: A selector with a default admits the entry

- **WHEN** a fixture table shaped like `triage` whose `implement` and `review` selectors carry a `default` case is run with `--enter verify`
- **THEN** the run is created, enters `verify` first, and review runs its `default` case with `phase/entered.case` reading `default`

#### Scenario: A dialect step that needs a change no result carries is refused

- **WHEN** a fixture table listing `implement, verify, analyze, review, ship, done, stop`, whose `analyze` seat opens with a dialect `check` step expanding `{change}`, both of its steps written `gate`, and declares no `change` input, is run with `--enter analyze`
- **THEN** no run is created, and the refusal names the `check` step of `analyze` as a dialect step expanding `{change}` ahead of any result that carries one (fact 6)

#### Scenario: Every shipped table's entries are pinned

- **WHEN** every recipe under `recipes/` is compiled and each of its phases is tried as an entry
- **THEN** the admitted set of each is pinned by a test, in `phases` order: `verify, review` in the `fast`-shaped tables (`fast`, `brokkr-fast`, `node`, `release`, `standby`, `review-first`, `wager-harness`, `wager-harness-dsh`, `wager-harness-muse`), in `panel-review` and `self`, and in `preflight` and `verify`; `classify, verify, review` in `landing`; `verify` in `research` and `research-dsh`; and none in `triage`, `gpt-flash` and `night-shift`; and every refused phase is pinned with the first fact it fails

### Requirement: A return hands its finding to a seat the run has not visited

In an entered run, a seat SHALL receive `returned_from`, the ruling's phase and the result that sent the run there, on its first visit when the ruling that sent it is a return, a ruling whose `next` stands before its `from` in the table's `phases` list, the order `context.results` is read in. A forward ruling into a phase this run has not visited SHALL hand nothing new, as a first visit does today; a revisit SHALL hand what it always did; triage SHALL receive nothing on any visit (decision 0041 ruling 6). The rule SHALL read the ruling's direction and never the entry's position. No seat input of a run without an entry SHALL move.

#### Scenario: The entered run's first return carries the finding

- **WHEN** a `fast`-shaped recipe is entered at `verify` and verify reports `fail`
- **THEN** the implementer's first visit receives `returned_from` with phase `verify` and the verify seat's result, and its `context.results` holds no result of a phase this run never ran

#### Scenario: A forward ruling into an unvisited phase hands nothing new

- **WHEN** a `fast`-shaped recipe is entered at `review`, review returns the run to `implement` by `REVIEW-REFORGE`, and the implementer completes
- **THEN** the implementer's first visit receives `returned_from` with phase `review`; verify's first visit, reached forward by `IMPL-OK`, receives no `returned_from`; and review's second visit receives `returned_from` with phase `verify`, as it does in any run

#### Scenario: A run without an entry builds the inputs it always built

- **WHEN** a `landing` run returns to `implement` from `review` for the first time, and a `fast` run returns to `implement` from `verify` for the second time
- **THEN** each seat input, and its digest, is byte for byte what the same journal produced before this change: the landing's first return still carries no `returned_from`, and the fast run's revisit carries it
