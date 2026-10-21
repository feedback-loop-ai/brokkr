## ADDED Requirements

### Requirement: A run enters at a gate of its table

`brokkr run` SHALL accept `--enter <phase>`, and a run started with it SHALL enter the named phase first, instead of the table's `initial`. The bundle it runs under SHALL be the one the same recipe compiles to without an entry: the table, its `initial`, the manifest, its digest and the witnesses do not move, because an entry is a fact of the run, not of the recipe. A recipe whose own `initial` is a gate SHALL start as it always did when no entry is named, and its `initial` SHALL be an entry like any other phase when it passes admission. A Looper-bound run (`--dispatch`) SHALL refuse an entry by name, as it refuses a named map, until the v2 manifest lineage carries one.

#### Scenario: A finished branch is verified, judged and shipped without a work seat

- **WHEN** a recipe whose table lists `intake, design, implement, verify, review, ship, done, stop`, intake and design being work phases, and which binds no library charter, is run with `--enter verify` over a repository whose branch holds the work a stopped run of the same recipe committed after entering intake, design and implement, and its verify, review and ship seats report `pass`, `clean`, `ready` and `shipped`
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
2. its name matches `^[a-z0-9][a-z0-9._-]{0,63}$`, the grammar the contribution gate prints (`run-continuation`), so the compiler never admits an entry the gate refuses; the table loader bounds no phase name, so this fact is the compiler's alone;
3. every driver-bearing site of that seat is a gate (its class under decision 0021 ruling 1 is `gate`; a sequence counts by its steps, each written `gate`, a dialect step included, since an undeclared site is work; a panel by its members; and a selector by every case and its default);
4. the protected phase (the bundle's `protected_phase`, `review` unless it names another) is the entry itself or is unavoidable from it, by the walk the compiler already makes from `initial`, started at the entry;
5. no work phase (a seated phase fact 3 would refuse) is met by the forward walk from the entry: a ruling is forward when its `next` stands after its `from` in the `phases` list, a return when before, a self-loop neither, and a park draws no edge;
6. no seat, step or ruling group the walk meets needs a journal fact that only a phase the walk does not enter can set. Three such facts exist, and no others:
   - `strategy`, read by a seat whose body selects on it with no `default` (`SeatBody::selected`, which has no body for an absent strategy), met without entering the strategy phase, `triage`, whose ruling alone sets it (decision 0041 ruling 6); the entry's own seat is met first;
   - `strategy`, read by a ruling: a `(phase, result)` of a seat met without entering the strategy phase, whose rulings that carry no `strategy_in` condition do not rule every valuation the sweep of decision 0050 ruling 4 puts to them, the sweep taken over those rulings alone, since an absent `strategy` satisfies no `strategy_in` and an unmatched result parks as unruled;
   - the dialect's `{change}`, expanded by a dialect step with no source the walk can supply. A source is an earlier step of the step's own sequence when its seat declares the `change` input (`carry_change`); else a result in `context.results` of a phase that is one of the five names `nearest_change` reads (`tasks`, `design`, `specify`, `triage`, `intake`, one roster held in one place and read by the engine and the admission alike, decision 0071 ruling 5), listed before the step's phase in `phases`, whose seat declares `change`; and the walk supplies that source only when every road from the entry to the step's phase enters such a phase, which is to say the step's phase is unreachable from the entry once those phases are removed, the shape of fact 4's walk.

The walk SHALL be breadth-first from the entry: the entry's own seat first; then, phase by phase in the order first reached, every ruling of that phase in the table's rule order; a park draws no edge, a terminal is reached but not entered, and a phase is visited once. Fact 5 SHALL follow forward rulings alone; fact 6 SHALL follow every ruling but never enter the phase that sets the fact. The witness a refusal names SHALL be the first site the walk meets that fails the fact: for fact 5 the work phase and the ruling that reached it; for fact 6 the seat, the step or the `(phase, result)`, the fact it needs, and the ruling that reached its phase, or none when it is the entry's own seat.

Any other entry SHALL be refused when the recipe and the entry are compiled, before a run id is minted or an event is written, by a typed error that names the phase, the first fact it failed with its witness, and the entries the table admits, derived by the same six facts and listed in the order of the `phases` list, or that it admits none. The engine SHALL hold no roster of which phases may be entered: the facts are the table's and its seats'.

#### Scenario: A work phase is refused

- **WHEN** a `fast`-shaped recipe (`implement, verify, review, ship, done, stop`) is run with `--enter implement`
- **THEN** no run is created, and the refusal names `implement` as a work phase (fact 3) and lists `verify, review` as the entries the table admits, in that order

#### Scenario: A gate that reaches done without the protected phase is refused

- **WHEN** a `fast`-shaped recipe is run with `--enter ship`
- **THEN** no run is created, and the refusal names `ship` as a gate from which `done` is reached without `review` (fact 4), and lists `verify, review`

#### Scenario: The protected phase itself is an entry

- **WHEN** a `fast`-shaped recipe is run with `--enter review` and review reports `clean`
- **THEN** the run enters `review` first, ship's drift check reads the head review was entered at, the journal holds no `phase/entered` for `verify`, and `run/started.entry.phase` reads `review`

#### Scenario: A phase the table lacks, a terminal, or a name the gate cannot print is refused

- **WHEN** a `fast`-shaped recipe is run with `--enter intake`, or with `--enter done`, or a fixture table whose gate is named `verify gate`, which the loader accepts, is run with `--enter "verify gate"`
- **THEN** no run is created, and the refusal names the phase as absent from the table (fact 1), as a terminal with no seat (fact 1), or as a name outside `^[a-z0-9][a-z0-9._-]{0,63}$` (fact 2), and lists the entries the table admits

#### Scenario: A dialect phase whose check step declares no class is a work phase

- **WHEN** a `triage`-shaped recipe is run with `--enter clarify`, or with `--enter analyze`
- **THEN** no run is created, and the refusal names the phase as a work phase, its `check` step declaring no class (fact 3), and says the table admits no entry

#### Scenario: A gate from which a work phase is reached forward is refused

- **WHEN** a `triage`-shaped recipe is run with `--enter triage`, or a fixture table shaped like it whose dialect steps are written `gate` is run with `--enter clarify` or `--enter analyze`
- **THEN** no run is created, and the refusal names `implement` reached forward by `TRIAGE-CHORE`, triage's first ruling in table order; `design` reached forward by `CLARIFY-CLEAR`, since `CLARIFY-AMBIGUOUS-EXHAUSTED` parks and `CLARIFY-AMBIGUOUS` returns; or `implement` reached forward by `ANALYZE-CONSISTENT`, the three drift rulings returning (fact 5), and says the table admits no entry

#### Scenario: A road that needs a skipped phase's strategy is refused

- **WHEN** a `triage`-shaped recipe is run with `--enter verify`
- **THEN** no run is created, and the refusal names `implement`, reached by `VERIFY-FAIL` after `VERIFY-FAIL-EXHAUSTED` reached only `stop`, as a seat that selects on `strategy` with no `default` while `triage`, which rules it, is not entered (fact 6), and says the table admits no entry
- **AND WHEN** it is run with `--enter review`
- **THEN** no run is created, and the refusal names `review`, the entry's own seat, reached by no ruling, for the same fact

#### Scenario: A selector with a default admits the entry

- **WHEN** a fixture table shaped like `triage` whose `implement` and `review` selectors carry a `default` case is run with `--enter verify`
- **THEN** the run is created, enters `verify` first, and review runs its `default` case with `phase/entered.case` reading `default`; and the entries the fixture admits are pinned as `verify, review`, since every `strategy_in` ruling of its `review` stands above an unconditioned ruling and every dialect step's source is entered on every road

#### Scenario: A ruling group only a skipped strategy decides is refused

- **WHEN** a fixture table shaped like `triage`, its selectors carrying a `default` case, whose `review` phase rules `clean` by one ruling conditioned on `strategy_in` over the whole strategy vocabulary and no other, which compiles because the sweep over present strategies is total, is run with `--enter verify`
- **THEN** no run is created, and the refusal names `(review, clean)`, reached by `VERIFY-PASS`, as a ruling group that no ruling free of `strategy_in` rules while `triage` is not entered (fact 6)

#### Scenario: A dialect step that needs a change no road carries is refused

- **WHEN** a fixture table listing `intake, implement, verify, analyze, review, ship, done, stop`, whose `intake` seat is work and declares the `change` input and whose `analyze` seat opens with a dialect `check` step expanding `{change}`, both of its steps written `gate`, and declares no `change` input, is run with `--enter analyze`
- **THEN** no run is created, and the refusal names the `check` step of `analyze` as a dialect step expanding `{change}` whose one source, `intake`, no road from the entry enters (fact 6)
- **AND WHEN** it is run with `--enter verify`
- **THEN** no run is created, and the refusal names the same step, reached by `VERIFY-PASS`, for the same fact, since the road from `verify` to `analyze` enters no `intake`

#### Scenario: Every shipped table's entries are pinned

- **WHEN** every recipe under `recipes/` is compiled and each of its phases is tried as an entry
- **THEN** the admitted set of each is pinned by a test, in `phases` order: `verify, review` in the `fast`-shaped tables (`fast`, `brokkr-fast`, `node`, `release`, `standby`, `review-first`, `wager-harness`, `wager-harness-dsh`, `wager-harness-muse`), in `panel-review` and `self`, and in `preflight` and `verify`; `classify, verify, review` in `landing`; `verify` in `research` and `research-dsh`; and none in `triage`, `gpt-flash` and `night-shift`; and every refused phase is pinned with the first fact it fails and the witness the walk names

### Requirement: An entry at the protected phase is the operator's word

An entry at the protected phase SHALL be admitted wherever the six facts admit it, `landing` included, and the run's own journal then holds no verify: it vouches a head no seat of that journal tested, which is the operator's word, as the chain is (`run-continuation`). The record SHALL say so on every surface: `run/started.entry.phase`, `context.entry`, the run view and the contribution gate's check line all name the gate entered. The engine SHALL read nothing of a continued run's verify. The by-hand road SHALL be unchanged: a landing started without an entry classifies the branch, and a branch with one code path still routes to verify by `CLASSIFY-CODE` (decision 0051 ruling 3).

#### Scenario: A landing entered at review over a code branch

- **WHEN** `brokkr run --recipe landing --enter review` is started over a branch that changes a code path, and review reports `clean`
- **THEN** the run is created, enters `review` first, holds no `phase/entered` for `classify` or `verify`, records `review` in `run/started.entry.phase`, and ships by `SHIP-COMPLETE`
- **AND WHEN** the same recipe is started without `--enter` over the same branch
- **THEN** `classify` rules `code` and the run enters `verify` by `CLASSIFY-CODE`, as it does today

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
