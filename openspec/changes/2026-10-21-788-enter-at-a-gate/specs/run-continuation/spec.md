## ADDED Requirements

### Requirement: An entered run names the runs it continues

`brokkr run --enter` SHALL accept `--continues <run>`, repeatable, naming the earlier runs whose work on the branch the entered run continues; without `--enter` the flag SHALL be refused. Each value SHALL resolve as `--run` does, a full id, a unique prefix or `latest`, in the journal the new run is written to. Each named run MUST exist there and be concluded, `completed` or `stopped`; a `running` run and an `awaiting_operator` run SHALL be refused naming the run and its status, since the one is still at work and the other is resumed or concluded first (decision 0092 ruling 4). Each named run's id MUST match the native run-id grammar the contribution gate reads, `^[a-z0-9-]{0,32}-[0-9a-f]{8}$`, the shape the engine mints, so a start admits no chain the gate refuses: an id outside it SHALL be refused naming the id and the grammar. Each named run MUST NOT be Looper-bound: a run whose start pinned the Looper-bound run-manifest/v2 lineage, its pinned run manifest (the `manifest` its `run/started` records and the store holds) carrying `schema` `forge-run-manifest/v2`, SHALL be refused naming the run and that lineage, whatever its id's shape. A Looper-bound run is told apart by what its start pinned, never by its id: its id is the dispatch's `forge_run_id`, any string of 1 to 512 characters (`contracts/dispatch-envelope.v2.schema.json`), which may match the native grammar, so a Looper id outside the grammar is refused by the grammar and one inside it by its lineage. A run named twice SHALL be refused. The checks SHALL be made for each id in the order named, in this order: named twice, absent from the journal, outside the grammar, Looper-bound, not concluded; the first that fails is the refusal. Every refusal SHALL come before an event is written. The chain SHALL be the operator's word: existence, conclusion, the grammar of the id and the lineage its start pinned are checked and nothing else, no realm, repository, recipe or verify is compared, and every surface SHALL say the chain as named at start. An entry that names no run SHALL be admitted: a branch with no run to continue, entered at a gate, is a landing.

#### Scenario: Two stopped runs are continued

- **WHEN** runs `a` and `b` stopped after committing their work, and a run is started with `--enter verify --continues a --continues b`
- **THEN** the run starts, and its `run/started.entry.continues` lists `a` then `b`, each with the feature its own start recorded

#### Scenario: A run of another recipe or realm is continued on the operator's word

- **WHEN** `--continues` names a concluded run of the journal that ran another recipe, or pinned another realm
- **THEN** the run starts and its entry records that run as named, with no refusal and no claim beyond the naming

#### Scenario: A parked or running run is not continued

- **WHEN** `--continues` names a run that is `awaiting_operator`, or one that is `running`
- **THEN** no run is created, and the refusal names the run, its status, and that it is resumed or concluded first

#### Scenario: A run named twice, or not at all

- **WHEN** `--continues` names the same run twice, or names a selector no run in the journal matches
- **THEN** no run is created, and the refusal names the selector and why

#### Scenario: A continued run the gate could not read is refused

- **WHEN** `--continues` names a concluded run journaled under the Looper wire id `forge-bound-run`, which is outside the native grammar
- **THEN** no run is created, and the refusal names the id and `^[a-z0-9-]{0,32}-[0-9a-f]{8}$`, before an event is written

#### Scenario: A Looper-bound run is not continued, whatever its id's shape

- **WHEN** `--continues` names a concluded run started with `--dispatch` under the Looper wire id `story-120-0a1b2c3d`, which matches `^[a-z0-9-]{0,32}-[0-9a-f]{8}$`, its start having pinned a run manifest whose `schema` is `forge-run-manifest/v2`
- **THEN** no run is created, and the refusal names `story-120-0a1b2c3d` as a Looper-bound run, its start pinning the run-manifest/v2 lineage, which a chain may not name, before an event is written

### Requirement: The journal records the entry and the chain

`run/started` SHALL carry two optional fields. `entry`, an object holding `phase`, the phase entered, and `continues`, the continued runs in the order named, each with its `run_id` and `feature`; `continues` is always present, an empty array when no run was named, so no reader tells an empty chain from an absent one by a missing key; `entry` SHALL be absent on a run started without an entry. `phases`, the compiled table's `phases` list verbatim and in its order, the list the engine reads `context.results`, the return rule and the admission's walk by, SHALL be written on every start, the Looper-bound one included, so a reader that holds no bundle reads the order the engine rules by. A run started without an entry then carries `feature`, `manifest`, `charters`, `layers` where the bundle composed layers, and `phases`, and no `entry` key. Every journal written before SHALL fold, verify and view as it did. The engine change SHALL move no witness: no manifest digest, no table digest and no charter digest under `crates/brokkr-runtime/tests/witnesses.json`; the charter edit of `An entered run's review judges the whole branch` moves, as any charter edit does, the `charters` rows of the five edited files and the `bundles` rows of every bundle whose composed seats bind one of them or pin one of the two copies, and nothing else. Both SHALL be typed once, at the edge, each through one reader that tells the key absent from a value that does not read, and the fold SHALL read nothing of either: `RunState` gains nothing, and each is an additive, optional, absent-by-default field under the vocabulary's amended rule. A present `entry` that does not read SHALL be refused by the engine where it reads it, at start and at resume; the run view, which refuses no journal the fold accepts, SHALL state it unreadable with the reader's error and repair nothing (`The run view names the gate entered and the runs continued`). `phases` is read by the view alone, which refuses no journal the fold accepts: a present `phases` that is not a list of strings SHALL read as none recorded, the rail standing in first-entry order (`operator-console-read`), and nothing SHALL repair it. The engine SHALL read `entry` at start and at resume to enter the entry instead of `initial`, so a run journaled but not yet entered resumes into its entry, and SHALL read `phases` from no journal, since it holds the table. Their numbered extension schema SHALL be minted when decision 0094 is accepted (decision 0082 ruling 1); until then the decision and the types are their shape.

#### Scenario: An ordinary run's start carries the order and no entry

- **WHEN** the acceptance fixture recipe, which binds no library charter, is started twice on the same tree, once without `--enter` and once with `--enter verify`
- **THEN** the first run's `run/started` carries `feature`, `manifest`, `charters` and `phases`, reading `intake, design, implement, verify, review, ship, done, stop`, the table's list verbatim, and no `entry` key; the second's carries the same `phases` and `entry`; and both pin the same manifest digest
- **AND** the witness table moves only in the rows the charter edit names: the `charters` rows of `reviewer.md`, `review-correctness.md`, `review-security.md`, `review-adversarial.md` and `node-reviewer.md`, and the `bundles` rows of every bundle but `preflight`, `research`, `research-dsh` and `verify`; every `tables` row, every other `charters` row and those four `bundles` rows stand at their values before this change

#### Scenario: An entry that names no run records an empty chain

- **WHEN** a run is started with `--enter review` and no `--continues`
- **THEN** its `run/started.entry` reads `review` as the phase and holds `continues` as an empty array, present

#### Scenario: A run journaled but not entered resumes into its entry

- **WHEN** an entered run's journal ends at `run/started`, the process having ended before `phase/entered`, and the run is resumed under its pinned bundle
- **THEN** the resumed run enters the entry phase, not `initial`

#### Scenario: An old journal folds as it did

- **WHEN** a journal written before `entry` and `phases` existed is folded, verified and viewed
- **THEN** it folds, verifies and views exactly as before, with no entry shown and its rail in first-entry order

#### Scenario: A recorded order the view cannot read is none

- **WHEN** a journal whose `run/started.phases` is a string, or a list holding a number, is folded, verified and viewed
- **THEN** it folds and verifies as any journal does, and its rail stands in first-entry order exactly as a journal that recorded no order, the view repairing nothing

### Requirement: The seats of an entered run are told the entry and the chain

Every seat input of an entered run except triage's SHALL carry `context.entry`: `phase`, the gate entered, and `continues`, the continued runs in the order named, each with `run_id` and `feature`, derived from `run/started.entry` alone so recovery rebuilds the same digest. Triage reads no journal, no fleet and no history (decision 0041 ruling 6), and the chain is history: its input SHALL carry no `context.entry`, as it carries no `last_decision` and no `returned_from`. No seat input of a run without an entry SHALL carry it. The continued runs' results SHALL NOT be handed: `context.results` holds only results of phases this run ran.

#### Scenario: The reviewer is handed the chain

- **WHEN** a run entered at `verify` continuing run `a` reaches `review`
- **THEN** the review seat's input carries `context.entry` with phase `verify` and `continues` listing `a` by id and feature, and its `context.results` holds verify's result and nothing of run `a`'s

#### Scenario: Triage is told nothing

- **WHEN** the fixture table shaped like `triage` with `default` cases on its selectors is entered at `verify` continuing run `a`, verify reports `fail`, and the implementer reports `oversized`, so the run reaches `triage` by `VERIFY-FAIL` then `IMPL-OVERSIZED`
- **THEN** the implementer's input carried `context.entry`, and triage's input carries no `context.entry`, no `last_decision` and no `returned_from`

#### Scenario: An ordinary seat input carries no entry

- **WHEN** a run started without `--enter` reaches any seat
- **THEN** its input carries no `context.entry`, and its digest is the digest the same journal produced before this change

### Requirement: An entered run's review judges the whole branch

The five charters whose scope names the run's beginning, `agents/charters/reviewer.md`, `review-correctness.md`, `review-security.md`, `review-adversarial.md` and `node-reviewer.md`, and the two copies pinned to the first, `recipes/review-first/roles/reviewer.md` and `recipes/standby/roles/reviewer.md`, SHALL scope the delivery as everything on the branch since its merge-base with the default branch, the cut the anchor's patch map takes and the gate vouches (decision 0038 ruling 1), and SHALL say that `context.entry` means the branch holds the work of the runs it names and all of it is the seat's to judge. The two copies SHALL stay byte for byte their source. A library test SHALL hold those seven files to the clause by path, and SHALL hold every charter under `agents/charters/` and every `roles/*.md` under `recipes/` to one textual rule: none scopes its read to the run's beginning or the pre-run commit. `verify-reviewer.md`, which reviews the range the feature text names, `review-spec-compliance.md`, which judges the change's whole diff, and `recipes/preflight/roles/reviewer.md`, which reads `main...HEAD`, the merge-base cut already, SHALL NOT change and need not name `context.entry`: the rule alone holds them.

#### Scenario: Every review charter names the base

- **WHEN** the library's charters and the recipe roles are read
- **THEN** each of the five charters and both copies names the merge-base with the default branch and `context.entry`; no file under `agents/charters/` or `recipes/*/roles/` scopes its read to the run's beginning or the pre-run commit; and `verify-reviewer.md`, `review-spec-compliance.md` and `recipes/preflight/roles/reviewer.md` are the bytes they were

#### Scenario: A copy that drifts from its source is refused

- **WHEN** `recipes/standby/roles/reviewer.md` or `recipes/review-first/roles/reviewer.md` differs from `agents/charters/reviewer.md`
- **THEN** the library test fails naming the copy and its source

### Requirement: The run view names the gate entered and the runs continued

`RunView` SHALL derive once, from `run/started`, the entry: the gate and the continued runs, each led by its title with its id after, as named at start, as parts and as one sentence, `entered at <phase>, continuing ` followed by `<title> (<id>)` for each run in the order named, joined by `, `, or `entered at <phase>, continuing no earlier run`. A continued run's title SHALL be `brokkr_view::title` of its recorded feature, the house's one derivation of a run's one-line name (#491): the feature's first line that says anything, trimmed, clamped at a word to 60 display columns with an ellipsis, the whole of `title`'s output, nothing substituted. Every one-line surface SHALL lead each continued run with that title: the sentence `brokkr run` prints on the line after `run started:`, the console's run header, `brokkr ui`'s header and the TUI's header line, which `graph_rows` counts as one row; each prints the sentence on one line however many lines a feature holds. The parts SHALL carry each continued run's whole `feature` beside its title, as a fleet row carries its `title` beside the whole `feature`, and every whole-text surface SHALL keep the feature whole: the journal's `run/started.entry`, `context.entry` and `brokkr inspect --json`'s `entry`. The console's run header, `brokkr ui` and the TUI SHALL paint that derivation and derive nothing of their own. `brokkr run` SHALL keep its start line `run started: <id>` exactly as it is, the id alone after the prefix, and SHALL print the sentence on the line after it, on the same stream, so the helpers and scripts that take everything after `run started: ` as the id read what they read; for a run without an entry nothing SHALL be painted and no line printed. A present `entry` the reader cannot read SHALL be stated, never painted as an ordinary run (the view's law 2: a refusal is reported, never repaired): the view's `entry` SHALL say it is unreadable with the reader's error, its one line `entry unreadable: <the reader's error>`, painted where the sentence would stand on every surface that paints the entry, and the rest of the run SHALL be painted as the fold gives it. The fleet list is unchanged.

#### Scenario: The header names the chain

- **WHEN** the run view of a run entered at `verify` continuing runs `a` and `b` is read on any surface
- **THEN** it names `verify` as the gate entered and the two runs, each by its title and then its id, in the order named

#### Scenario: A multi-line feature is led by its title on one line

- **WHEN** run `a` recorded a feature of three lines, the first `land the parked slice`, the second `It stopped at REVIEW-REFORGE-EXHAUSTED with every commit on the branch.` and the third `Judge the whole branch again.`, and `brokkr run --enter verify --continues a` starts
- **THEN** the line after `run started: ` reads `entered at verify, continuing land the parked slice (a)`, one line; the run view's sentence, the console's header, `brokkr ui`'s header and the TUI's header line read the same, the TUI's `graph_rows` counting it as one row; and `run/started.entry.continues[0].feature`, the reviewer's `context.entry` and `brokkr inspect --json`'s `entry` carry the three lines whole, the view's parts holding the title `land the parked slice` beside them

#### Scenario: An entry the view cannot read is stated

- **WHEN** a journal whose `run/started.entry` is the string `verify`, which the fold accepts, is viewed
- **THEN** the view's `entry` states the entry unreadable with the reader's error, its line reading `entry unreadable: ` and that error, which the console's header, `brokkr ui` and the TUI paint where the sentence would stand; no gate entered and no continued run is shown; and the rail, the roads back, the participants and the timeline read as the fold gives them

#### Scenario: A run entered at a gate over a branch with no run

- **WHEN** the run view of a run entered at `review` with no `--continues` is read
- **THEN** it names `review` as the gate entered and says the run continues no earlier run

#### Scenario: The start line keeps its form

- **WHEN** `brokkr run --enter verify --continues a` starts, run `a` having recorded the feature `the first slice`
- **THEN** the line beginning `run started: ` carries the new run's id and nothing else, and the next line reads `entered at verify, continuing the first slice (a)`
- **AND WHEN** the same recipe is started without `--enter`
- **THEN** the start line is printed alone

### Requirement: The contribution gate reads the chain and still vouches

`scripts/delivered-by-brokkr.sh` SHALL vouch an entered run exactly as it vouches any run: the ship anchor's per-file patch map is cut from the merge-base of the default branch and the vouched head, so it spans every commit on the branch whichever run or hand made it, and the tier is cut by the delta since that judgment (decision 0038). The gate SHALL read `entry` from the vouching run's verified journal and name the gate entered and each continued run's id on its check line, never a feature text. An `entry` whose shape it cannot read, not an object, a `phase` outside `^[a-z0-9][a-z0-9._-]{0,63}$` (the grammar compile admission applies), or `continues` not an array of objects whose `run_id` matches `^[a-z0-9-]{0,32}-[0-9a-f]{8}$` (the grammar the launch applies), SHALL be a refusal, never a vouch; an absent `entry` SHALL read as today. One test SHALL hold the script's two patterns equal to the compiler's and the launch's. The gate SHALL fetch no continued run's evidence and SHALL claim no verify it did not read: a run entered at the protected phase vouches as any run does, and its line names the gate entered.

#### Scenario: An entered run vouches for a branch two runs built

- **WHEN** run `a` committed an implementation and stopped, run `b` entered at `verify` continuing `a` remediated once and shipped, and the pull request head is the head `b` anchored
- **THEN** the gate passes at tier `vouched` and its line names run `b`, the gate `verify`, and run `a` by id

#### Scenario: A run entered at review vouches, and the line says so

- **WHEN** run `b` entered at `review`, its journal holding no verify, judged the branch clean and shipped, and the pull request head is the head it anchored
- **THEN** the gate passes at tier `vouched`, its line names run `b` and the gate `review`, and the gate fetched nothing of any continued run

#### Scenario: A head edited after the entered run's judgment

- **WHEN** a code path is committed after run `b` shipped
- **THEN** the gate refuses with the code delta, as it would for any run

#### Scenario: A chain the gate cannot read

- **WHEN** the vouching run's `run/started.entry` is a string, names a phase outside the grammar, or lists a continued run whose `run_id` is outside the native grammar
- **THEN** the gate refuses, naming the run and the unreadable entry, and the operator's by-hand label is the only way past, logged as every skip is
