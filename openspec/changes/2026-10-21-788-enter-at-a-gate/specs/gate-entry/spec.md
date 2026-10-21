## ADDED Requirements

### Requirement: A run enters at a gate of its table

`brokkr run` SHALL accept `--enter <phase>`, and a run started with it SHALL enter the named phase first, instead of the table's `initial`. The bundle it runs under SHALL be the one the same recipe compiles to without an entry: the table, its `initial`, the manifest, its digest and the witnesses do not move, because an entry is a fact of the run, not of the recipe. A recipe whose own `initial` is a gate SHALL start as it always did when no entry is named, and its `initial` SHALL be an entry like any other phase when it passes admission. The compiled `landing` SHALL list `classify` first, as its own table writes it: its bundle marks `phases` in `override.table`, beside `initial`, so its own list, which names every phase it inherits, replaces the inherited one, as `triage`'s and `panel-review`'s bundles mark theirs, the union composition keeping the base's order and appending a derived recipe's own phase after it (decision 0017; D36, D37); nothing else of the recipe moves, and its seats listed after `classify` then read `classify`'s result in `context.results`, as every seat reads the results of the phases listed before it (`A return hands its finding to a seat the run has not visited`; D38). A Looper-bound run (`--dispatch`) SHALL refuse an entry by name, as it refuses a named map and in the same place, first in the launch, before a recipe is resolved, a bundle compiled, an envelope read or a journal opened, until the v2 manifest lineage carries one. `--enter` and `--continues` SHALL be `brokkr run`'s alone: `brokkr queue add`, which shares the rest of the launch's arguments, SHALL refuse either as an argument it does not know and queue nothing; and `QueuedLaunch::of` SHALL refuse a `NewRun` that carries an entry, naming the entry phase and `queued-launch/v1`, before anything is encoded or written, as it refuses a named map beside a dispatch, so `queued-launch/v1` is not widened.

#### Scenario: A finished branch is verified, judged and shipped without a work seat

- **WHEN** a recipe whose table lists `intake, design, implement, verify, review, ship, done, stop`, intake and design being work phases, and which binds no library charter, is run with `--enter verify` over a repository whose branch holds the work a stopped run of the same recipe committed after entering intake, design and implement, and its verify, review and ship seats report `pass`, `clean`, `ready` and `shipped`
- **THEN** the run completes by `SHIP-COMPLETE`; its `phase/entered` events name `verify`, `review` and `ship` and never `intake`, `design` or `implement`; no effect is requested of a work seat; and the run manifest equals the manifest a run of the same recipe without an entry pins

#### Scenario: A landing starts where it always did, and its first phase is an entry

- **WHEN** `brokkr run --recipe landing` is started without `--enter`
- **THEN** the run enters `classify` first and its `run/started` carries no `entry` and records `phases` as `classify, implement, verify, review, ship, done, stop`: the compiled `landing` lists its phases as its own table writes them, `classify` first, not after the phases it inherits, its bundle marking `phases` in `override.table` (D36, D37), and `landing`'s shape test pins that order
- **AND WHEN** `classify` rules `code` and the run enters `verify` by `CLASSIFY-CODE`
- **THEN** `verify`'s input holds `classify`'s result in `context.results` and, the run having no entry, no `returned_from` (D38); and the run view, reading the recorded order, calls `CLASSIFY-CODE` forward and draws no road back for it, where the order composition gave the recipe drew one (D36's F3)
- **AND WHEN** it is started with `--enter classify --continues <a stopped run>`
- **THEN** the run enters `classify` first and its `run/started.entry` records `classify` and the run named
- **AND WHEN** `classify` rules `code` in that entered run
- **THEN** `Direction::of` reads `CLASSIFY-CODE` forward over the compiled order, and `verify`'s first visit carries `context.entry`, `classify`'s result and no `returned_from`, where the order composition gave the recipe, `classify` listed last, would have handed it classify's result as a return (D39)

#### Scenario: A Looper-bound run takes no entry

- **WHEN** `brokkr run` is given both `--dispatch` and `--enter`
- **THEN** the launch is refused naming both flags before anything is compiled, opened or written

#### Scenario: A queued launch takes no entry

- **WHEN** `brokkr queue add` is given `--enter verify`, or `--continues <run>`, beside the arguments a launch takes
- **THEN** the command refuses the argument as one it does not know, and the queue holds no new entry
- **AND WHEN** a `NewRun` carrying an entry at `verify` is handed to `QueuedLaunch::of`
- **THEN** it refuses, naming `verify` and `queued-launch/v1`, and nothing is encoded or written

### Requirement: Entry is admitted by six facts of the table and its seats

An entry SHALL be admitted when the compiled table and the entry satisfy all six, judged in this order:

1. the phase is one of the table's `phases` and is not a terminal, so it is seated;
2. its name matches `^[a-z0-9][a-z0-9._-]{0,63}$`, the grammar the contribution gate prints (`run-continuation`), so the compiler never admits an entry the gate refuses; the table loader bounds no phase name, so this fact is the compiler's alone;
3. every driver-bearing site of that seat is a gate (its class under decision 0021 ruling 1 is `gate`; a single counts by its class; a sequence by its steps, each written `gate`, a dialect step included, since an undeclared site is work; a panel by its members; and a selector by every case and its default). This is the seat's compiled whole-gate fact, `has_gate`, the fact the dispatch door journals as the effect's `gate`, and no derived step class decides it. A dialect step counts by the class it wrote whether or not the realm's dialect supplies it, so a step the dialect does not supply, for which no site is compiled, still fails this fact when it wrote no `gate`;
4. the protected phase (the bundle's `protected_phase`, `review` unless it names another) is the entry itself or is unavoidable from it, by the walk the compiler already makes from `initial`, started at the entry;
5. no work phase (a seated phase fact 3 would refuse) is met by the forward walk from the entry: a ruling is forward when its `next` stands after its `from` in the `phases` list, a return when before, a self-loop neither, and a park draws no edge, by the one predicate `brokkr-core` holds, `Direction::of`, which the return rule below and the run view's roads back (`operator-console-read`) read as well, so nothing else decides a direction;
6. no seat, step or ruling group the walk meets needs a journal fact that only a phase the walk does not enter can set. Three such facts exist, and no others:
   - `strategy`, read by a seat whose body selects on it with no `default` (`SeatBody::selected`, which has no body for an absent strategy), met without entering the strategy phase, `triage`, whose ruling alone sets it (decision 0041 ruling 6); the entry's own seat is met first;
   - `strategy`, read by a ruling: a `(phase, result)` of a seat met without entering the strategy phase that holds any ruling with a `strategy_in` condition, whatever its other rulings rule. An absent `strategy` satisfies no `strategy_in`, so in the entered run such a ruling never matches where a run started at `initial` may match it, and the group could decide otherwise than that run would: a hard denial a strategy decides would fall through to an unconditioned allow below it. The admission preserves every ruling the skipped phase could decide; it does not stop at proving the group total (D36, the operator's ruling of 2026-10-22). A reached phase that holds several such groups is witnessed by the first by result in key order, the order `Machine::groups` builds a table's `(phase, result)` groups in and the sweep reports an unruled group in, which the core function answering those groups keeps (D41). A group with no `strategy_in` ruling decides as a run started at `initial` decides it, and an unmatched result parks as unruled there as here. An entry that is itself the strategy phase sets `strategy` by its own ruling before any other phase is met, so no seat or ruling group is met without entering it: both `strategy` facts hold and no walk is taken for them. That entry is admitted exactly as a plain start at `triage` is, and the admission refuses no more and no less: a `triage` ruling outside the strategy vocabulary sets no `strategy`, and `escalate`, which no selector case names, selects none, so a later selector with no `default` has no body, as in a run started at `triage` today, which no compile refusal guards;
   - the dialect's `{change}`, expanded by a dialect step with no source the walk can supply. A source is an earlier step of the step's own sequence when its seat declares the `change` input (`carry_change`); else a result in `context.results` of a phase that is one of the five names `nearest_change` reads (`tasks`, `design`, `specify`, `triage`, `intake`, one roster held in one place and read by the engine and the admission alike, decision 0071 ruling 5), listed before the step's phase in `phases`, whose seat declares `change`; and the walk supplies that source only when every road from the entry to the step's phase enters such a phase, which is to say the step's phase is unreachable from the entry once those phases are removed, the shape of fact 4's walk. An entry that is itself one of those phases supplies the source, since the run enters its entry first, so the step is covered and no walk is taken for it.

The walk SHALL be breadth-first from the entry: the entry's own seat first; then, phase by phase in the order first reached, every ruling of that phase in the table's rule order; a park draws no edge, a terminal is reached but not entered, and a phase is visited once. Fact 5 SHALL follow forward rulings alone. Fact 6 SHALL be its three facts judged in their listed order, each over its own walk: the selector fact and the ruling fact, for an entry that is not the strategy phase, over one walk that follows every ruling but never enters the strategy phase; the `{change}` fact over one walk per dialect step whose sources do not include the entry, the steps taken in the order of the `phases` list and, within a seat, in step order, that follows every ruling but never enters a source phase of that step, so a step whose phase that walk reaches has a road no source lies on. The witness a refusal names SHALL be the first failing fact's first witness in its own walk's order: for fact 3 the first site whose class is work in the seat's written order, a selector's cases in key order and then its default, a sequence's steps in written step order, a panel's members in key order, a dialect step taken by the class it wrote and a step the dialect does not supply standing in that order and named by its step name and written class as uncompiled, so the witness is never the first compiled work site when an uncompiled step stands before it; for fact 5 the work phase and the ruling that reached it; for fact 6 the seat, the step or the `(phase, result)`, a phase's groups taken by result in key order (D41), the fact it needs, and the ruling that reached its phase, or none when it is the entry's own seat. A table that fails two facts of six SHALL name the one listed first, and the next refusal names the next.

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

#### Scenario: A dialect step the realm does not supply is named by the class it wrote

- **WHEN** a fixture table listing `clarify, review, done`, `review` protected and written `gate`, is compiled under a realm whose dialect is `speckit`, whose `clarify.check` is `unsupported`, and its `clarify` seat is a sequence of a dialect `check` step declaring no class, for which no site is compiled, and a `judge` step written `gate`, its only compiled site, and the recipe is run with `--enter clarify`
- **THEN** no run is created, and the refusal names `clarify` as a work phase, its witness the `check` step, uncompiled because `speckit` does not supply a clarify check, by the class it wrote, none, an undeclared site being work (fact 3), not the `judge`; and it lists `review` as the entry the table admits
- **AND WHEN** the `check` step is written `gate`, the dialect still supplying none
- **THEN** fact 3 holds, and the entries the fixture admits are pinned as `clarify, review`

#### Scenario: A gate from which a work phase is reached forward is refused

- **WHEN** a `triage`-shaped recipe is run with `--enter triage`, or a fixture table shaped like it whose dialect steps are written `gate` is run with `--enter clarify` or `--enter analyze`
- **THEN** no run is created, and the refusal names `implement` reached forward by `TRIAGE-CHORE`, triage's first ruling in table order; `design` reached forward by `CLARIFY-CLEAR`, since `CLARIFY-AMBIGUOUS-EXHAUSTED` parks and `CLARIFY-AMBIGUOUS` returns; or `implement` reached forward by `ANALYZE-CONSISTENT`, the three drift rulings returning (fact 5), and says the table admits no entry

#### Scenario: A road that needs a skipped phase's strategy is refused

- **WHEN** a `triage`-shaped recipe is run with `--enter verify`
- **THEN** no run is created, and the refusal names `implement`, reached by `VERIFY-FAIL` after `VERIFY-FAIL-EXHAUSTED` reached only `stop`, as a seat that selects on `strategy` with no `default` while `triage`, which rules it, is not entered (fact 6), and says the table admits no entry
- **AND WHEN** it is run with `--enter review`
- **THEN** no run is created, and the refusal names `review`, the entry's own seat, reached by no ruling, for the same fact
- **AND WHEN** a table shaped like it whose `implement` is a plain work seat and whose `review` still selects with no `default` (`night-shift`) is run with `--enter verify`
- **THEN** no run is created, and the refusal names `review`, reached by `VERIFY-PASS`, for the same fact, `implement`, reached first by `VERIFY-FAIL`, having needed nothing

#### Scenario: A judging phase with a work site is a work phase, and a gate ahead of it is refused forward

- **WHEN** a table shaped like `triage` whose `review` cases are each a sequence of a `positions` panel whose members are written `work` and a `chief` written `gate` (`gpt-flash`) is run with `--enter review`
- **THEN** no run is created, and the refusal names `review` as a work phase, its first work site the `correctness` member of the `positions` step of its `chore` case (fact 3), and says the table admits no entry
- **AND WHEN** it is run with `--enter verify`
- **THEN** no run is created, and the refusal names `review`, reached forward by `VERIFY-PASS`, as a work phase (fact 5), no fact 6 having been judged, and says the table admits no entry

#### Scenario: A selector with a default admits the entry

- **WHEN** a fixture table shaped like `triage` whose `implement` and `review` selectors carry a `default` case, and none of whose rulings past `triage` carries a `strategy_in` condition, is run with `--enter verify`
- **THEN** the run is created, enters `verify` first, and review runs its `default` case with `phase/entered.case` reading `default`; and the entries the fixture admits are pinned as `verify, review`, since no ruling group the walk meets reads `strategy` and every dialect step's source is entered on every road

#### Scenario: A ruling group only a skipped strategy decides is refused

- **WHEN** a fixture table shaped like `triage`, its selectors carrying a `default` case, whose `review` phase rules `clean` by one ruling conditioned on `strategy_in` over the whole strategy vocabulary and no other, which compiles because the sweep over present strategies is total, is run with `--enter verify`
- **THEN** no run is created, and the refusal names `(review, clean)`, reached by `VERIFY-PASS`, as a ruling group that reads `strategy`, holding a `strategy_in` ruling, while `triage` is not entered (fact 6; D36, D40)

#### Scenario: A hard denial a skipped strategy decides is refused, an unconditioned ruling below it notwithstanding

- **WHEN** a fixture table whose selectors carry a `default` case, and whose `review` rules `clean` first by `DENY-ENGINE` (`strategy_in: [engine]`, severity `hard`, next `stop`) and then by an unconditioned ruling to `ship`, a table a run started at `initial` with strategy `engine` stops on and the sweep finds total, is run with `--enter verify` or `--enter review`
- **THEN** no run is created, and the refusal names `(review, clean)`, reached by `VERIFY-PASS` from `verify` and by no ruling from `review`, as a ruling group that reads `strategy` while `triage` is not entered (fact 6), and the table admits neither entry
- **AND WHEN** the same table without `DENY-ENGINE` is run with `--enter verify`
- **THEN** the run is created

#### Scenario: Two ruling groups of one phase read strategy, and the first by result is named

- **WHEN** the fixture table of `A hard denial a skipped strategy decides is refused, an unconditioned ruling below it notwithstanding`, whose `review` also reports `residual` and rules it first by `DENY-DESIGN-RESIDUAL` (`strategy_in: [design]`, severity `hard`, next `stop`) and then by an unconditioned ruling to `implement`, the two `residual` rulings written ahead of the two `clean` rulings in the table's rule order, is run with `--enter verify`
- **THEN** no run is created, and the refusal names `(review, clean)`, reached by `VERIFY-PASS`, and not `(review, residual)`, which stands first in rule order and second by result: a reached phase's groups that read `strategy` are named by result in key order, the order `Machine::groups` builds them in and the sweep reports them in (fact 6, D41)
- **AND WHEN** the two `clean` rulings are written ahead of the two `residual` rulings
- **THEN** the refusal names `(review, clean)` still

#### Scenario: A dialect step that needs a change no road carries is refused

- **WHEN** a fixture table listing `intake, implement, verify, analyze, review, ship, done, stop`, whose `intake` seat is work and declares the `change` input and whose `analyze` seat opens with a dialect `check` step expanding `{change}`, both of its steps written `gate`, and declares no `change` input, is run with `--enter analyze`
- **THEN** no run is created, and the refusal names the `check` step of `analyze` as a dialect step expanding `{change}` whose one source, `intake`, no road from the entry enters (fact 6)
- **AND WHEN** it is run with `--enter verify`
- **THEN** no run is created, and the refusal names the same step, reached by `VERIFY-PASS`, for the same fact, since the road from `verify` to `analyze` enters no `intake`

#### Scenario: An entry that is a step's change source covers the step

- **WHEN** a fixture table listing `implement, design, analyze, review, ship, done, stop`, whose one work seat is `implement`, reached from `design` only by a return, whose `design` seat is a gate declaring the `change` input, and whose `analyze` seat opens with a dialect `check` step expanding `{change}`, both of its steps written `gate`, and declares no `change` input, is run with `--enter design`
- **THEN** the run is created and enters `design` first: the entry is the step's one source, so the step is covered and no walk is taken for it (fact 6)

#### Scenario: An entry at the strategy phase holds both strategy facts

- **WHEN** a fixture table listing `triage, verify, review, ship, done, stop`, whose `triage` seat is a gate ruling forward into `verify` and whose `review` seat selects on `strategy` with no `default`, every case a gate, is run with `--enter triage`
- **THEN** the run is created and enters `triage` first: its own ruling sets `strategy` before any other phase is met, so both strategy facts hold and no walk is taken for them (fact 6)
- **AND WHEN** it is run with `--enter verify`
- **THEN** no run is created, and the refusal names `review`, reached by `VERIFY-PASS`, as a seat that selects on `strategy` with no `default` while `triage` is not entered (fact 6)

#### Scenario: Two facts of six fail, and the one listed first is named

- **WHEN** the fixture table of the scenario `A dialect step that needs a change no road carries is refused`, with a strategy phase `triage` listed ahead of `intake` that rules forward into it and declares no `change`, and with a `review` seat that selects on `strategy` with no `default`, is run with `--enter verify`
- **THEN** no run is created, and the refusal names `review`, reached by `ANALYZE-CONSISTENT`, as a seat that selects on `strategy` with no `default` while `triage` is not entered, the first fact of six, and not the `check` step of `analyze`, which the change walk reaches first, by `VERIFY-PASS`
- **AND WHEN** the fixture's `review` selector gains a `default` case and it is run with `--enter verify`
- **THEN** no run is created, and the refusal names the `check` step of `analyze`, reached by `VERIFY-PASS`, as a dialect step expanding `{change}` whose one source, `intake`, no road from the entry enters

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

In an entered run, a seat SHALL receive `returned_from`, the ruling's phase and the result that sent the run there, on its first visit when the ruling that sent it is a return, a ruling whose `next` stands before its `from` in the table's `phases` list, the order `context.results` is read in, as `Direction::of` reads it, the one predicate the admission's fact 5 and the run view's roads back read. A forward ruling into a phase this run has not visited SHALL hand nothing new, as a first visit does today; a revisit SHALL hand what it always did; triage SHALL receive nothing on any visit (decision 0041 ruling 6). The rule SHALL read the ruling's direction and never the entry's position. No seat input of a run without an entry SHALL move by the engine change: it carries no `context.entry`, and `returned_from` reaches it as it always did. One recipe's inputs move, by its compiled order and not by the engine: `landing`'s seats listed after `classify`, the remediating `implement`, `verify`, `review` and `ship`, SHALL read `classify`'s result in `context.results`, as every seat reads the results of the phases listed before it in `phases`, which the order composition gave the recipe, `classify` listed last, denied them; each such input differs from what the same journal produced before this change by that one member and by nothing else, and a landing in flight when the marker lands is refused at resume by its moved manifest, as any recipe edit refuses a mid-flight run (D38).

#### Scenario: The entered run's first return carries the finding

- **WHEN** a `fast`-shaped recipe is entered at `verify` and verify reports `fail`
- **THEN** the implementer's first visit receives `returned_from` with phase `verify` and the verify seat's result, and its `context.results` holds no result of a phase this run never ran

#### Scenario: A forward ruling into an unvisited phase hands nothing new

- **WHEN** a `fast`-shaped recipe is entered at `review`, review returns the run to `implement` by `REVIEW-REFORGE`, and the implementer completes
- **THEN** the implementer's first visit receives `returned_from` with phase `review`; verify's first visit, reached forward by `IMPL-OK`, receives no `returned_from`; and review's second visit receives `returned_from` with phase `verify`, as it does in any run

#### Scenario: A run without an entry builds the inputs it always built

- **WHEN** a `fast` run returns to `implement` from `verify` for the second time, and a `landing` run returns to `implement` from `review` for the first time
- **THEN** the fast run's revisit input, and its digest, is byte for byte what the same journal produced before this change, `returned_from` carried as it always was; and the landing's first return still carries no `returned_from`, its input differing from what the same journal produced before this change by `context.results.classify` alone, the result of the phase its compiled order now lists first (D38)
